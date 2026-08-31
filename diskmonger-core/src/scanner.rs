//! Contient la logique de scan du système de fichiers de manière récursive.
use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::fs;
use std::path::Path;

// Flag global atomique pour l'annulation de l'analyse.
pub static CANCEL_SCAN: AtomicBool = AtomicBool::new(false);

// Importation nécessaire pour la détection du Device ID et de l'Inode sur Unix.
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub const MAX_RECURSION_DEPTH: u32 = 64;

#[derive(Clone, serde::Serialize)]
pub struct ScanProgressPayload {
  pub path: String,
  pub files: u64,
  pub dirs: u64,
}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FsFile {
    pub name: Box<str>,
    pub size_in_clusters: u32,
}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FsDir {
    pub name: Box<str>,
    pub size_in_clusters: u32,
    pub children: Vec<FsNode>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FsNode {
    File(FsFile),
    Dir(FsDir),
}

impl FsNode {
    pub fn size_in_clusters(&self) -> u32 {
        match self {
            FsNode::File(f) => f.size_in_clusters,
            FsNode::Dir(d) => d.size_in_clusters,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            FsNode::File(f) => &f.name,
            FsNode::Dir(d) => &d.name,
        }
    }
    pub fn is_directory(&self) -> bool {
        matches!(self, FsNode::Dir(_))
    }
    pub fn count_items(&self) -> usize {
        match self {
            FsNode::File(_) => 1,
            FsNode::Dir(dir) => dir.children.iter().map(FsNode::count_items).sum::<usize>() + 1,
        }
    }
}

// --- OS-Specific Logic ---
#[cfg(windows)]
fn get_cluster_size(path: &Path) -> u64 {
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::GetDiskFreeSpaceW;
    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut sectors_per_cluster = 0;
    let mut bytes_per_sector = 0;
    unsafe {
        if GetDiskFreeSpaceW(path_wide.as_ptr(), &mut sectors_per_cluster, &mut bytes_per_sector, &mut Default::default(), &mut Default::default()) != 0 {
            (sectors_per_cluster as u64) * (bytes_per_sector as u64)
        } else { 4096 }
    }
}
#[cfg(unix)]
fn get_cluster_size(path: &Path) -> u64 {
    use std::os::unix::ffi::OsStrExt;
    use libc::statvfs;
    use std::ffi::CString;
    let c_path = CString::new(path.as_os_str().as_bytes()).unwrap();
    let mut stats: statvfs = unsafe { std::mem::zeroed() };
    if unsafe { statvfs(c_path.as_ptr(), &mut stats) } == 0 {
        stats.f_bsize as u64
    } else { 4096 }
}
#[cfg(not(any(windows, unix)))]
fn get_cluster_size(_path: &Path) -> u64 { 4096 }


/// Fonction récursive unifiée qui parcourt le système de fichiers.
fn scan_recursive(
    path: &Path,
    cluster_size: u64,
    progress: &Arc<Mutex<ScanProgressPayload>>,
    progress_callback: &mut Option<&mut dyn FnMut(&ScanProgressPayload)>,
    depth: u32,
    _initial_device: u64,
    _seen_inodes: &mut HashSet<u64>, // Suivi des inodes pour les hard links
) -> Result<FsNode, String> {
    if CANCEL_SCAN.load(Ordering::SeqCst) {
        return Err("Analyse annulée par l'utilisateur".to_string());
    }

    if depth > MAX_RECURSION_DEPTH {
        return Ok(FsNode::File(FsFile { name: "DEEPLY_NESTED_OR_LOOP".into(), size_in_clusters: 0 }));
    }

    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) => return Err(e.to_string()),
    };

    #[cfg(unix)]
    {
        if metadata.dev() != _initial_device {
            return Ok(FsNode::File(FsFile { name: path.file_name().unwrap_or_default().to_string_lossy().into(), size_in_clusters: 0 }));
        }
		// Si c'est un fichier avec plusieurs liens (hard link) et qu'on l'a déjà vu, on ne le compte pas.
        if metadata.is_file() && metadata.nlink() > 1 && !_seen_inodes.insert(metadata.ino()) {
             return Ok(FsNode::File(FsFile { name: path.file_name().unwrap_or_default().to_string_lossy().into(), size_in_clusters: 0 }));
        }
    }

    if metadata.file_type().is_symlink() {
        return Ok(FsNode::File(FsFile { name: path.file_name().unwrap_or_default().to_string_lossy().into(), size_in_clusters: 0 }));
    }

    let name: Box<str> = path.file_name().unwrap_or_default().to_string_lossy().into();

    let mut trigger_callback = false;
    if !metadata.is_dir() {
        {
            let mut progress_guard = progress.lock().unwrap();
            progress_guard.files += 1;
            let total_items = progress_guard.files + progress_guard.dirs;
            if total_items % 100 == 0 {
                progress_guard.path = path.to_string_lossy().to_string();
                trigger_callback = true;
            }
        }
        
        let logical_size = metadata.len();
        let size_in_clusters = ((logical_size + cluster_size - 1) / cluster_size) as u32;
        
        if trigger_callback {
            if let Some(cb) = progress_callback {
                let progress_val = progress.lock().unwrap().clone();
                cb(&progress_val);
            }
        }
        
        return Ok(FsNode::File(FsFile { name, size_in_clusters }));
    }

    {
        let mut progress_guard = progress.lock().unwrap();
        progress_guard.dirs += 1;
        let total_items = progress_guard.files + progress_guard.dirs;
        if total_items % 100 == 0 {
            progress_guard.path = path.to_string_lossy().to_string();
            trigger_callback = true;
        }
    }

    if trigger_callback {
        if let Some(cb) = progress_callback {
            let progress_val = progress.lock().unwrap().clone();
            cb(&progress_val);
        }
    }

    let mut children = Vec::new();
    let mut total_size_in_clusters: u32 = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry_result in entries {
            if let Ok(entry) = entry_result {
                if let Ok(child_node) = scan_recursive(&entry.path(), cluster_size, progress, progress_callback, depth + 1, _initial_device, _seen_inodes) {
                    total_size_in_clusters = total_size_in_clusters.saturating_add(child_node.size_in_clusters());
                    children.push(child_node);
                }
            }
        }
    }

    Ok(FsNode::Dir(FsDir { name, size_in_clusters: total_size_in_clusters, children }))
}

/// Point d'entrée pour lancer un scan complet.
pub fn scan_directory(
    path_str: &str,
    progress_callback: Option<&mut dyn FnMut(&ScanProgressPayload)>,
) -> Result<(FsNode, u64), String> {
    let path = Path::new(path_str);
    if !path.exists() {
        return Err(format!("Le chemin n'existe pas : {}", path_str));
    }
    let mount_point = path.ancestors().last().unwrap_or(path);
    let cluster_size = get_cluster_size(mount_point);

    let progress = Arc::new(Mutex::new(ScanProgressPayload {
        path: path_str.to_string(),
        files: 0,
        dirs: 0,
    }));

    let mut seen_inodes: HashSet<u64> = HashSet::new();

    #[cfg(unix)]
    let initial_device = match fs::metadata(path) {
        Ok(m) => m.dev(),
        Err(e) => return Err(e.to_string()),
    };

    #[cfg(not(unix))]
    let initial_device = 0;

    let mut progress_cb = progress_callback;
    let mut root_node = scan_recursive(
        path,
        cluster_size,
        &progress,
        &mut progress_cb,
        0,
        initial_device,
        &mut seen_inodes,
    )?;

    match &mut root_node {
        FsNode::Dir(d) => d.name = path_str.to_string().into(),
        FsNode::File(f) => f.name = path_str.to_string().into(),
    }

    Ok((root_node, cluster_size))
}

/// Retourne l'espace disque libre en octets pour le disque contenant le chemin donné.
pub fn get_free_space(path: String) -> Result<u64, String> {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        let c_path = CString::new(path.as_bytes()).map_err(|e| e.to_string())?;
        let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(c_path.as_ptr(), &mut stats) } == 0 {
            Ok((stats.f_bfree as u64) * (stats.f_frsize as u64))
        } else {
            Err("Impossible de récupérer les informations du système de fichiers.".to_string())
        }
    }
    #[cfg(windows)]
    {
        use winapi::um::fileapi::GetDiskFreeSpaceExW;
        use winapi::um::winnt::ULARGE_INTEGER;
        use std::os::windows::ffi::OsStrExt;
        
        let path_wide: Vec<u16> = std::path::Path::new(&path).as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        
        let mut free_bytes_to_caller = unsafe { std::mem::zeroed::<ULARGE_INTEGER>() };
        let mut total_bytes = unsafe { std::mem::zeroed::<ULARGE_INTEGER>() };
        let mut total_free_bytes = unsafe { std::mem::zeroed::<ULARGE_INTEGER>() };
        
        if unsafe { GetDiskFreeSpaceExW(path_wide.as_ptr(), &mut free_bytes_to_caller, &mut total_bytes, &mut total_free_bytes) } != 0 {
            Ok(unsafe { *free_bytes_to_caller.QuadPart() })
        } else {
            Err("Impossible de récupérer l'espace disque libre.".to_string())
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        Ok(0)
    }
}

/// Vérifie si le chemin donné correspond au point de montage d'une partition.
pub fn is_mount_point(path: String) -> Result<bool, String> {
    #[cfg(windows)]
    {
        use std::path::Path;
        let p = Path::new(&path);
        if !p.is_absolute() {
            return Ok(false);
        }
        Ok(p.parent().map_or(false, |parent| parent.as_os_str() == p.as_os_str()))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if path == "/" {
            return Ok(true);
        }
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => return Ok(false),
        };
        let mut parent_path = std::path::PathBuf::from(&path);
        if !parent_path.pop() {
            return Ok(false);
        }
        let parent_meta = match std::fs::metadata(parent_path) {
            Ok(m) => m,
            Err(_) => return Ok(false),
        };
        Ok(metadata.dev() != parent_meta.dev())
    }
    #[cfg(not(any(unix, windows)))]
    {
        Ok(false)
    }
}

/// Récupère l'espace libre uniquement si l'on se trouve à la racine de la partition scannée.
pub fn get_view_free_space(current_zoom_path: &std::path::Path, scan_root_path_str: &str) -> Option<u64> {
    if current_zoom_path == std::path::Path::new(scan_root_path_str) {
        if is_mount_point(scan_root_path_str.to_string()).unwrap_or(false) {
            get_free_space(scan_root_path_str.to_string()).ok()
        } else {
            None
        }
    } else {
        None
    }
}

/// Recherche de manière récursive et efficace un nœud mutable (`FsNode`) dans l'arbre à partir de son chemin absolu.
pub fn find_node_mut<'a>(
    node: &'a mut FsNode,
    target_path: &std::path::Path,
    current_path: &std::path::Path,
) -> Option<&'a mut FsNode> {
    if current_path == target_path {
        return Some(node);
    }
    if let FsNode::Dir(dir) = node {
        for child in &mut dir.children {
            let child_path = current_path.join(child.name());
            if target_path.starts_with(&child_path) {
                if let Some(found) = find_node_mut(child, target_path, &child_path) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Recharge le cache pour un sous-dossier spécifique au sein de l'arbre principal `root_node`.
pub fn refresh_subfolder_cache(
    root_node: &mut FsNode,
    subfolder_path: &str,
    scan_root_path: &str,
    progress_callback: Option<&mut dyn FnMut(&ScanProgressPayload)>,
) -> Result<(), String> {
    // 1. Scanne le sous-dossier de manière isolée
    let (mut new_node, _) = scan_directory(subfolder_path, progress_callback)?;

    // 2. Corrige le nom du nœud scanné pour qu'il soit l'identifiant individuel (et non le chemin complet)
    let individual_name = std::path::Path::new(subfolder_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    match &mut new_node {
        FsNode::Dir(d) => d.name = individual_name.into(),
        FsNode::File(f) => f.name = individual_name.into(),
    }

    // 3. Remplace l'ancien nœud par le nouveau dans l'arbre principal
    let target_path = std::path::Path::new(subfolder_path);
    let root_path = std::path::Path::new(scan_root_path);

    if let Some(node_ref) = find_node_mut(root_node, target_path, root_path) {
        *node_ref = new_node;
        Ok(())
    } else {
        Err(format!("Impossible de trouver le sous-dossier dans le cache : {}", subfolder_path))
    }
}

//! Contient la logique de scan du système de fichiers de manière récursive.
use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::fs;
use std::path::Path;
use tauri::{Window, Emitter};

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
    window: &Window,
    progress: &Arc<Mutex<ScanProgressPayload>>,
    depth: u32,
    _initial_device: u64,
    _seen_inodes: &mut HashSet<u64>, // Suivi des inodes pour les hard links
) -> Result<FsNode, String> {
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

    if !metadata.is_dir() {
        progress.lock().unwrap().files += 1;
        let logical_size = metadata.len();
        let size_in_clusters = ((logical_size + cluster_size - 1) / cluster_size) as u32;
        return Ok(FsNode::File(FsFile { name, size_in_clusters }));
    }
    
    {
        let mut progress_guard = progress.lock().unwrap();
        progress_guard.dirs += 1;
        if progress_guard.dirs % 500 == 0 {
            progress_guard.path = path.to_string_lossy().to_string();
            let _ = window.emit("scan-progress", progress_guard.clone());
        }
    }

    let mut children = Vec::new();
    let mut total_size_in_clusters: u32 = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry_result in entries {
            if let Ok(entry) = entry_result {
                if let Ok(child_node) = scan_recursive(&entry.path(), cluster_size, window, progress, depth + 1, _initial_device, _seen_inodes) {
                    total_size_in_clusters = total_size_in_clusters.saturating_add(child_node.size_in_clusters());
                    children.push(child_node);
                }
            }
        }
    }

    Ok(FsNode::Dir(FsDir { name, size_in_clusters: total_size_in_clusters, children }))
}

/// Point d'entrée pour lancer un scan complet.
pub fn scan_directory(path_str: &str, window: &Window) -> Result<(FsNode, u64), String> {
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

    let mut root_node = scan_recursive(path, cluster_size, window, &progress, 0, initial_device, &mut seen_inodes)?;

    match &mut root_node {
        FsNode::Dir(d) => d.name = path_str.to_string().into(),
        FsNode::File(f) => f.name = path_str.to_string().into(),
    }
    
    Ok((root_node, cluster_size))
}

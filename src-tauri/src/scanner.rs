//! Ce module contiendra la logique de scan du système de fichiers.

use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use tauri::{Window, Emitter};
use treemap::Rect;

const SMALL_FILE_THRESHOLD_RATIO: f64 = 0.005; // 0.5%

// --- Structures de Données ---

#[derive(Clone, serde::Serialize)]
struct ScanProgressPayload {
  path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FsEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_directory: bool,
    pub children: Vec<FsEntry>,
    #[serde(skip)]
    pub bounds: Rect,
}

// --- Logique de Scan ---

#[cfg(windows)]
fn get_cluster_size(path: &Path) -> u64 {
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::GetDiskFreeSpaceW;
    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut sectors_per_cluster = 0;
    let mut bytes_per_sector = 0;
    let mut number_of_free_clusters = 0;
    let mut total_number_of_clusters = 0;
    unsafe {
        if GetDiskFreeSpaceW(path_wide.as_ptr(), &mut sectors_per_cluster, &mut bytes_per_sector, &mut number_of_free_clusters, &mut total_number_of_clusters) != 0 {
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
    unsafe {
        if statvfs(c_path.as_ptr(), &mut stats) == 0 { stats.f_bsize as u64 } else { 4096 }
    }
}

#[cfg(not(any(windows, unix)))]
fn get_cluster_size(_path: &Path) -> u64 { 4096 }

fn scan_recursive(path: &Path, cluster_size: u64, window: &Window) -> Result<FsEntry, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    if !metadata.is_dir() {
        let logical_size = metadata.len();
        let allocated_size = (logical_size + cluster_size - 1) / cluster_size * cluster_size;
        return Ok(FsEntry {
            name,
            path: path.to_string_lossy().to_string(),
            size: allocated_size,
            is_directory: false,
            children: vec![],
            bounds: Rect::new(),
        });
    }
    
    let _ = window.emit("scan-progress", ScanProgressPayload { path: path.to_string_lossy().to_string() });

    let mut children = Vec::new();
    let mut total_size: u64 = 0;
    match fs::read_dir(path) {
        Ok(entries) => {
            for entry_result in entries {
                if let Ok(entry) = entry_result {
                    if let Ok(child_entry) = scan_recursive(&entry.path(), cluster_size, window) {
                        total_size += child_entry.size;
                        children.push(child_entry);
                    }
                }
            }
        }
        Err(e) => { return Err(e.to_string()); }
    }

    // --- Regroupement des petits fichiers ---
    let threshold = (total_size as f64 * SMALL_FILE_THRESHOLD_RATIO) as u64;
    let mut processed_children: Vec<FsEntry> = Vec::new();
    let mut small_files_size: u64 = 0;

    for child in children {
        if child.size < threshold && !child.is_directory {
            small_files_size += child.size;
        } else {
            processed_children.push(child);
        }
    }

    if small_files_size > 0 {
        processed_children.push(FsEntry {
            name: "[Autres fichiers]".to_string(),
            path: path.to_string_lossy().to_string(),
            size: small_files_size,
            is_directory: false,
            children: vec![],
            bounds: Rect::new(),
        });
    }

    Ok(FsEntry {
        name,
        path: path.to_string_lossy().to_string(),
        size: total_size,
        is_directory: true,
        children: processed_children,
        bounds: Rect::new(),
    })
}

pub fn scan_directory(path_str: &str, window: &Window) -> Result<FsEntry, String> {
    let path = Path::new(path_str);
    if !path.exists() {
        return Err(format!("Le chemin n'existe pas : {}", path_str));
    }
    let mount_point = path.ancestors().last().unwrap_or(path);
    let cluster_size = get_cluster_size(mount_point);
    scan_recursive(path, cluster_size, window)
}

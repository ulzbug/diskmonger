//! Point d'entrée principal de l'application Tauri et définition des commandes IPC.

use diskmonger_core::{layout, scanner};

use tauri::{Emitter, Manager, State, Window};
use std::sync::{Arc, Mutex};
use std::thread;

/// Le cache applicatif, accessible par toutes les commandes.
/// Contient l'arbre de scan complet (`FsNode`) et la taille de cluster du système de fichiers.
/// `Arc<Mutex<>>` est utilisé pour un accès thread-safe depuis les différentes commandes.
pub struct ScanCache(pub Arc<Mutex<Option<(scanner::FsNode, u64)>>>);

/// Récupère la locale du système d'exploitation (ex: "fr", "en") pour l'i18n.
#[tauri::command]
fn get_locale() -> String {
    sys_locale::get_locale()
        .unwrap_or_else(|| "en".to_string())
        .split('-')
        .next()
        .unwrap_or("en")
        .to_string()
}

/// Retourne le chemin de scan par défaut selon l'OS (Dossier Documents sur Windows, Home sur les autres).
#[tauri::command]
fn get_default_scan_path(app: tauri::AppHandle) -> Result<String, String> {
    let path = if cfg!(target_os = "windows") {
        app.path().document_dir().map_err(|e| e.to_string())?
    } else {
        app.path().home_dir().map_err(|e| e.to_string())?
    };
    Ok(path.to_string_lossy().to_string())
}

/// Lance un scan de système de fichiers dans un thread séparé pour ne pas bloquer l'UI.
#[tauri::command]
fn scan(path: String, window: Window, cache: State<ScanCache>) {
    scanner::CANCEL_SCAN.store(false, std::sync::atomic::Ordering::SeqCst);

    let cache_clone = Arc::clone(&cache.0);
    let window_clone = window.clone();
    thread::Builder::new()
        .name("scanner-thread".to_string())
        .stack_size(32 * 1024 * 1024) 
        .spawn(move || {
            match scanner::scan_directory(&path, Some(&mut move |progress| {
                let _ = window_clone.emit("scan-progress", progress.clone());
            })) {
                Ok((entry, cluster_size)) => {
                    if scanner::CANCEL_SCAN.load(std::sync::atomic::Ordering::SeqCst) {
                        let _ = window.emit("scan-cancelled", ());
                    } else {
                        *cache_clone.lock().unwrap() = Some((entry, cluster_size));
                        let _ = window.emit("scan-complete", ());
                    }
                }
                Err(e) => {
                    if scanner::CANCEL_SCAN.load(std::sync::atomic::Ordering::SeqCst) {
                        let _ = window.emit("scan-cancelled", ());
                    } else {
                        let _ = window.emit("scan-error", e);
                    }
                }
            }
        })
        .unwrap();
}

/// Calcule et retourne le layout pour un chemin de segments donné.
#[tauri::command]
fn zoom_in(segments: Vec<String>, width: f64, height: f64, cache: State<ScanCache>, show_free_space: bool) -> Result<layout::LayoutResult, String> {
    let mut cache_lock = cache.0.lock().unwrap();
    if let Some((root_node, cluster_size)) = &mut *cache_lock {
        
        let root_name_str = root_node.name();
        let mut current_path = std::path::PathBuf::from(root_name_str);

        let mut should_add_free_space = false;
        if show_free_space && segments.is_empty() {
            if is_mount_point(root_name_str.to_string()).unwrap_or(false) {
                should_add_free_space = true;
            }
        }
        
        let free_space_bytes = if should_add_free_space {
            get_free_space(root_name_str.to_string()).ok()
        } else {
            None
        };

        let mut current_node = root_node;
        for segment in &segments {
            match current_node {
                scanner::FsNode::Dir(dir) => {
                    if let Some(next_node) = dir.children.iter_mut().find(|child| child.name() == segment) {
                        current_node = next_node;
                        current_path.push(segment);
                    } else {
                        return Err(format!("Segment de chemin non trouvé dans le cache : {}", segment));
                    }
                },
                scanner::FsNode::File(_) => return Err("Impossible de zoomer dans un fichier.".to_string()),
            }
        }
        
        let padding = layout::Padding {
            header: 12.0,
            sides: 4.0,
            bottom: 4.0,
        };
        Ok(layout::calculate_layout(current_node, width, height, *cluster_size, &current_path, free_space_bytes, padding, 0.0002))

    } else {
        Err("Aucun scan n'a été effectué".to_string())
    }
}

/// Déplace un fichier ou un dossier vers la corbeille du système.
#[tauri::command]
async fn trash_item(path: String) -> Result<(), String> {
    trash::delete(&path).map_err(|e| e.to_string())
}

/// Ouvre l'explorateur de fichiers du système et y sélectionne le fichier ou le dossier.
#[tauri::command]
async fn reveal_in_explorer(path: String) -> Result<(), String> {
    opener::reveal(&path).map_err(|e| e.to_string())
}

/// Signale au thread de scan de s'arrêter proprement de manière atomique.
#[tauri::command]
fn cancel_scan() {
    scanner::CANCEL_SCAN.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Retourne l'espace disque libre en octets pour le disque contenant le chemin donné.
#[tauri::command]
fn get_free_space(path: String) -> Result<u64, String> {
    scanner::get_free_space(path)
}

#[tauri::command]
fn is_mount_point(path: String) -> Result<bool, String> {
    scanner::is_mount_point(path)
}

#[tauri::command]
fn refresh_subfolder(path: String, cache: State<'_, ScanCache>, window: Window) -> Result<(), String> {
    let mut cache_lock = cache.0.lock().unwrap();
    if let Some((root_node, _cluster_size)) = cache_lock.as_mut() {
        let scan_root_path = root_node.name().to_string();
        let window_clone = window.clone();
        scanner::refresh_subfolder_cache(
            root_node,
            &path,
            &scan_root_path,
            Some(&mut move |progress| {
                let _ = window_clone.emit("scan-progress", progress.clone());
            }),
        )?;
    }
    Ok(())
}

/// Configure et lance l'application Tauri.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ScanCache(Default::default()))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan,
            zoom_in,
            get_default_scan_path,
            get_locale,
            trash_item,
            reveal_in_explorer,
            cancel_scan,
            get_free_space,
            is_mount_point,
            refresh_subfolder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

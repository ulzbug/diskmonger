//! Point d'entrée principal de l'application Tauri et définition des commandes IPC.

mod scanner;
mod layout;

use tauri::{Emitter, Manager, State};
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
/// Une fois terminé, met le résultat en cache et émet un événement `scan-complete` au frontend.
#[tauri::command]
fn scan(path: String, window: tauri::Window, cache: State<ScanCache>) {
    let cache_clone = Arc::clone(&cache.0);
    thread::Builder::new()
        .name("scanner-thread".to_string())
        .stack_size(32 * 1024 * 1024) // 32MB de stack pour les systèmes de fichiers très profonds
        .spawn(move || {
            match scanner::scan_directory(&path, &window) {
                Ok((entry, cluster_size)) => {
                    *cache_clone.lock().unwrap() = Some((entry, cluster_size));
                    let _ = window.emit("scan-complete", ());
                }
                Err(e) => {
                    let _ = window.emit("scan-error", e);
                }
            }
        })
        .unwrap();
}

/// Calcule et retourne le layout pour un chemin de segments donné.
/// C'est le point d'entrée pour l'affichage initial et pour chaque zoom.
#[tauri::command]
fn zoom_in(segments: Vec<String>, width: f64, height: f64, cache: State<ScanCache>) -> Result<layout::LayoutResult, String> {
    let mut cache_lock = cache.0.lock().unwrap();
    if let Some((root_node, cluster_size)) = &mut *cache_lock {
        
        let mut current_path = std::path::PathBuf::from(root_node.name());
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
        
        Ok(layout::calculate_layout(current_node, width, height, *cluster_size, &current_path))

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
            reveal_in_explorer
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

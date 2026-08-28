mod scanner;
mod layout;

use tauri::{Emitter, Manager, State};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct ScanCache(pub Arc<Mutex<Option<(scanner::FsNode, u64)>>>);

#[tauri::command]
fn get_locale() -> String {
    sys_locale::get_locale()
        .unwrap_or_else(|| "en".to_string())
        .split('-')
        .next()
        .unwrap_or("en")
        .to_string()
}

#[tauri::command]
fn get_default_scan_path(app: tauri::AppHandle) -> Result<String, String> {
    let path = if cfg!(target_os = "windows") {
        app.path().document_dir().map_err(|e| e.to_string())?
    } else {
        app.path().home_dir().map_err(|e| e.to_string())?
    };
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn scan(path: String, window: tauri::Window, cache: State<ScanCache>) {
    let cache_clone = Arc::clone(&cache.0);
    thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            match scanner::scan_directory(&path, &window) {
                Ok((entry, cluster_size)) => {
                    // On stocke simplement l'arbre et la taille du cluster en cache
                    *cache_clone.lock().unwrap() = Some((entry, cluster_size));
                    // On émet un signal de fin vide
                    window.emit("scan-complete", ()).unwrap();
                }
                Err(e) => {
                    window.emit("scan-error", e).unwrap();
                }
            }
        })
        .unwrap();
}

#[tauri::command]
fn zoom_in(segments: Vec<String>, width: f64, height: f64, cache: State<ScanCache>) -> Result<layout::LayoutResult, String> {
    let mut cache_lock = cache.0.lock().unwrap();
    if let Some((root_node, cluster_size)) = &mut *cache_lock {
        
        let mut current_node = root_node;
        for segment in &segments {
            match current_node {
                scanner::FsNode::Dir(dir) => {
                    if let Some(next_node) = dir.children.iter_mut().find(|child| child.name() == segment) {
                        current_node = next_node;
                    } else {
                        return Err(format!("Segment de chemin non trouvé dans le cache : {}", segment));
                    }
                },
                scanner::FsNode::File(_) => return Err("Impossible de zoomer dans un fichier.".to_string()),
            }
        }
        
        Ok(layout::calculate_layout(current_node, width, height, *cluster_size))

    } else {
        Err("Aucun scan n'a été effectué".to_string())
    }
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ScanCache(Default::default()))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![scan, zoom_in, get_default_scan_path, get_locale])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

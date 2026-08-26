mod scanner;
mod layout;

use tauri::{Emitter, State};
use std::sync::{Arc, Mutex};
use std::thread;

// Le cache global pour notre arborescence de fichiers, thread-safe.
pub struct ScanCache(pub Arc<Mutex<Option<scanner::FsEntry>>>);

#[tauri::command]
fn scan(path: String, width: f64, height: f64, window: tauri::Window, cache: State<ScanCache>) {
    let cache_clone = Arc::clone(&cache.0);

    thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            match scanner::scan_directory(&path, &window) {
                Ok(mut entry) => {
                    let layout = layout::calculate_layout(&mut entry, width, height);

                    *cache_clone.lock().unwrap() = Some(entry);

                    window.emit("scan-complete", layout).unwrap();
                }
                Err(e) => {
                    window.emit("scan-error", e).unwrap();
                }
            }
        })
        .unwrap();
}

#[tauri::command]
fn zoom_in(path: String, width: f64, height: f64, cache: State<ScanCache>) -> Result<Vec<layout::Rectangle>, String> {
    let mut cache_lock = cache.0.lock().unwrap();
    if let Some(root) = &mut *cache_lock {
        fn find_node<'a>(node: &'a mut scanner::FsEntry, path: &str) -> Option<&'a mut scanner::FsEntry> {
            if node.path == path { return Some(node); }
            for child in &mut node.children {
                if let Some(found) = find_node(child, path) {
                    return Some(found);
                }
            }
            None
        }

        if let Some(node_to_zoom) = find_node(root, &path) {
            Ok(layout::calculate_layout(node_to_zoom, width, height))
        } else {
            Err("Chemin non trouvé dans le cache".to_string())
        }
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
        .invoke_handler(tauri::generate_handler![scan, zoom_in])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

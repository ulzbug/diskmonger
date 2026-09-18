// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Vérifie la présence d'un environnement graphique sur Linux avant de lancer Tauri
    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
            eprintln!("\x1b[31mErreur : Impossible d'initialiser l'environnement graphique.\x1b[0m");
            eprintln!("DiskMonger Desktop nécessite un serveur d'affichage (X11 ou Wayland).");
            eprintln!("\nPour une utilisation en ligne de commande (sur un serveur, par exemple),");
            eprintln!("veuillez utiliser l'exécutable dédié : \x1b[32mdiskmonger-cli\x1b[0m\n");
            std::process::exit(1);
        }
    }

    diskmonger_lib::run()
}

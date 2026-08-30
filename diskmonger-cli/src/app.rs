use clap::ValueEnum;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use diskmonger_core::{layout, scanner};
use ratatui::prelude::*;
use std::{error::Error, io, time::Duration};

use crate::ui::ui;
use crate::utils::*;
use crate::i18n::t;

#[derive(Debug, Clone, ValueEnum)]
pub enum RenderStyle {
    Nested,
    Flat,
}

pub enum AppState {
    Browsing,
    InMenu,
}

pub struct App {
    pub state: AppState,
    pub selected_menu_index: usize,
    pub focused_path: Option<String>,
    pub show_free_space: bool,
}

impl Default for App {
    fn default() -> Self {
        App {
            state: AppState::Browsing,
            selected_menu_index: 0,
            focused_path: None,
            show_free_space: true,
        }
    }
}

/// Zoom récursivement dans le dossier actuellement sélectionné (focus).
pub fn perform_zoom_in(
    root_node: &mut scanner::FsNode,
    app: &mut App,
    current_zoom_path: &mut std::path::PathBuf,
    layout_result: &mut layout::LayoutResult,
    cluster_size: u64,
    scan_root_path_str: &str,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    if let Some(focused) = &app.focused_path {
        let target_path = std::path::Path::new(focused);
        let scan_root_path = std::path::Path::new(scan_root_path_str);

        let is_dir = if let Some(node) = find_node_mut(root_node, target_path, scan_root_path) {
            node.is_directory()
        } else {
            false
        };

        if is_dir {
            *current_zoom_path = target_path.to_path_buf();
            if let Some(new_zoom_node) = find_node_mut(root_node, current_zoom_path, scan_root_path) {
                let terminal_size = terminal.size()?;
                *layout_result = layout::calculate_layout(
                    new_zoom_node,
                    terminal_size.width as f64,
                    (terminal_size.height - 1) as f64,
                    cluster_size,
                    current_zoom_path,
                    if app.show_free_space {
                        scanner::get_view_free_space(current_zoom_path, scan_root_path_str)
                    } else {
                        None
                    },
                    layout::Padding { header: 1.0, sides: 1.0, bottom: 1.0 },
                    0.001,
                );
                app.focused_path = layout_result.rectangles.first().map(|r| r.path.clone());
            }
        }
    }
    Ok(())
}

/// Remonte d'un niveau de zoom (parent du zoom racine actuel).
pub fn perform_zoom_out(
    root_node: &mut scanner::FsNode,
    app: &mut App,
    current_zoom_path: &mut std::path::PathBuf,
    layout_result: &mut layout::LayoutResult,
    cluster_size: u64,
    scan_root_path_str: &str,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    let scan_root_path = std::path::Path::new(scan_root_path_str);
    if *current_zoom_path != scan_root_path {
        if let Some(new_zoom_path) = current_zoom_path.parent() {
            let old_zoom_path_str = current_zoom_path.to_string_lossy().to_string();
            *current_zoom_path = new_zoom_path.to_path_buf();
            if let Some(new_zoom_node) = find_node_mut(root_node, current_zoom_path, scan_root_path) {
                let terminal_size = terminal.size()?;
                *layout_result = layout::calculate_layout(
                    new_zoom_node,
                    terminal_size.width as f64,
                    (terminal_size.height - 1) as f64,
                    cluster_size,
                    current_zoom_path,
                    if app.show_free_space {
                        scanner::get_view_free_space(current_zoom_path, scan_root_path_str)
                    } else {
                        None
                    },
                    layout::Padding { header: 1.0, sides: 1.0, bottom: 1.0 },
                    0.001,
                );
                app.focused_path = Some(old_zoom_path_str);
            }
        }
    }
    Ok(())
}

/// Réinitialise le zoom au niveau du dossier racine scanné.
pub fn perform_zoom_reset(
    root_node: &mut scanner::FsNode,
    app: &mut App,
    current_zoom_path: &mut std::path::PathBuf,
    layout_result: &mut layout::LayoutResult,
    cluster_size: u64,
    scan_root_path_str: &str,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    let scan_root_path = std::path::Path::new(scan_root_path_str);
    if *current_zoom_path != scan_root_path {
        *current_zoom_path = scan_root_path.to_path_buf();
        let terminal_size = terminal.size()?;
        *layout_result = layout::calculate_layout(
            root_node,
            terminal_size.width as f64,
            (terminal_size.height - 1) as f64,
            cluster_size,
            current_zoom_path,
            if app.show_free_space {
                scanner::get_view_free_space(current_zoom_path, scan_root_path_str)
            } else {
                None
            },
            layout::Padding { header: 1.0, sides: 1.0, bottom: 1.0 },
            0.001,
        );
        app.focused_path = layout_result.rectangles.first().map(|r| r.path.clone());
    }
    Ok(())
}

/// Alterne l'affichage de l'espace disque libre pour la partition racine.
pub fn perform_toggle_free_space(
    root_node: &mut scanner::FsNode,
    app: &mut App,
    current_zoom_path: &std::path::Path,
    layout_result: &mut layout::LayoutResult,
    cluster_size: u64,
    scan_root_path_str: &str,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    app.show_free_space = !app.show_free_space;
    let scan_root_path = std::path::Path::new(scan_root_path_str);
    if let Some(zoom_node) = find_node_mut(root_node, current_zoom_path, scan_root_path) {
        let terminal_size = terminal.size()?;
        *layout_result = layout::calculate_layout(
            zoom_node,
            terminal_size.width as f64,
            (terminal_size.height - 1) as f64,
            cluster_size,
            current_zoom_path,
            if app.show_free_space {
                scanner::get_view_free_space(current_zoom_path, scan_root_path_str)
            } else {
                None
            },
            layout::Padding { header: 1.0, sides: 1.0, bottom: 1.0 },
            0.001,
        );
        if !layout_result.rectangles.iter().any(|r| Some(&r.path) == app.focused_path.as_ref()) {
            app.focused_path = layout_result.rectangles.first().map(|r| r.path.clone());
        }
    }
    Ok(())
}

/// Démarre la boucle d'événements interactive de l'application.
pub fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    root_node: &mut scanner::FsNode,
    cluster_size: u64,
    path: &str,
    style: &RenderStyle,
) -> Result<(), Box<dyn Error>> {
    let mut app = App::default();
    let is_partition = scanner::is_mount_point(path.to_string()).unwrap_or(false);
    let menu_keys_str = t("cli-menu-key");
    let menu_keys = menu_keys_str.as_str();

    // Analyse dynamique du raccourci de Menu basé sur le mnémonique &
    let (_, _, keys_shortcut) = parse_mnemonic(menu_keys);

    let terminal_rect = terminal.size()?;
    let mut current_zoom_path = std::path::PathBuf::from(path);
    let total_scanned_size = root_node.size_in_clusters() as u64 * cluster_size;
    let mut layout_result = layout::calculate_layout(
        root_node,
        terminal_rect.width as f64,
        (terminal_rect.height - 1) as f64,
        cluster_size,
        &current_zoom_path,
        if app.show_free_space { scanner::get_view_free_space(&current_zoom_path, path) } else { None },
        layout::Padding { header: 1.0, sides: 1.0, bottom: 1.0 },
        0.001,
    );

    if !layout_result.rectangles.is_empty() {
        app.focused_path = Some(layout_result.rectangles[0].path.clone());
    }

    loop {
        // Traductions dynamiques
        let title_zoom = t("cli-menu-zoom");
        let title_dezoom = t("cli-menu-dezoom");
        let title_reset = t("cli-menu-reset");
        let title_reload = t("cli-menu-reload");
        let title_toggle = if app.show_free_space { t("cli-menu-hide-free") } else { t("cli-menu-show-free") };
        let title_quit = t("cli-menu-quit");

        // Construction dynamique de menu_titles basé sur l'état show_free_space
        let mut menu_titles = vec![title_zoom.as_str(), title_dezoom.as_str(), title_reset.as_str(), title_reload.as_str()];
        if is_partition {
            menu_titles.push(title_toggle.as_str());
        }
        menu_titles.push(title_quit.as_str());

        // Analyse dynamique des raccourcis basés sur les mnémoniques &
        let parsed_menu: Vec<(String, Option<char>)> = menu_titles.iter().map(|&t| {
            let (clean, _, shortcut) = parse_mnemonic(t);
            (clean, shortcut)
        }).collect();

        terminal.draw(|f| {
            ui(f, &app, &layout_result.rectangles, style, &menu_titles, menu_keys, total_scanned_size);
        })?;

        if crossterm::event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
                        return Ok(());
                    }
                    match app.state {
                        AppState::Browsing => match key.code {
                            KeyCode::Tab | KeyCode::Right => {
                                if let Some(focused) = &app.focused_path {
                                    if let Some(siblings) = get_visible_siblings(focused, &layout_result.rectangles) {
                                        if let Some(idx) = siblings.iter().position(|p| p == focused) {
                                            let next_idx = (idx + 1) % siblings.len();
                                            app.focused_path = Some(siblings[next_idx].clone());
                                        }
                                    }
                                } else if !layout_result.rectangles.is_empty() {
                                    app.focused_path = Some(layout_result.rectangles[0].path.clone());
                                }
                            }
                            KeyCode::Left => {
                                if let Some(focused) = &app.focused_path {
                                    if let Some(siblings) = get_visible_siblings(focused, &layout_result.rectangles) {
                                        if let Some(idx) = siblings.iter().position(|p| p == focused) {
                                            let prev_idx = (idx + siblings.len() - 1) % siblings.len();
                                            app.focused_path = Some(siblings[prev_idx].clone());
                                        }
                                    }
                                } else if !layout_result.rectangles.is_empty() {
                                    app.focused_path = Some(layout_result.rectangles[0].path.clone());
                                }
                            }
                            KeyCode::Down => {
                                if let Some(focused) = &app.focused_path {
                                    if let Some(child_path) = get_first_visible_child_path(focused, &layout_result.rectangles) {
                                        app.focused_path = Some(child_path);
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if let Some(focused) = &app.focused_path {
                                    let target_path = std::path::Path::new(focused);
                                    if let Some(parent_path) = target_path.parent() {
                                        let parent_path_str = parent_path.to_string_lossy().to_string();
                                        let scan_root_path = std::path::Path::new(path);
                                        if target_path != scan_root_path {
                                            if parent_path == current_zoom_path {
                                                perform_zoom_out(
                                                    root_node,
                                                    &mut app,
                                                    &mut current_zoom_path,
                                                    &mut layout_result,
                                                    cluster_size,
                                                    path,
                                                    terminal,
                                                )?;
                                            } else {
                                                if layout_result.rectangles.iter().any(|r| r.path == parent_path_str) {
                                                    app.focused_path = Some(parent_path_str);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                perform_zoom_in(
                                    root_node,
                                    &mut app,
                                    &mut current_zoom_path,
                                    &mut layout_result,
                                    cluster_size,
                                    path,
                                    terminal,
                                )?;
                            }
                            KeyCode::Char(c) => {
                                let c_lower = c.to_ascii_lowercase();
                                if Some(c_lower) == keys_shortcut {
                                    app.state = AppState::InMenu;
                                } else if Some(c_lower) == parsed_menu[0].1 { // Zoomer
                                    perform_zoom_in(
                                        root_node,
                                        &mut app,
                                        &mut current_zoom_path,
                                        &mut layout_result,
                                        cluster_size,
                                        path,
                                        terminal,
                                    )?;
                                } else if Some(c_lower) == parsed_menu[1].1 { // Dézoomer
                                    perform_zoom_out(
                                        root_node,
                                        &mut app,
                                        &mut current_zoom_path,
                                        &mut layout_result,
                                        cluster_size,
                                        path,
                                        terminal,
                                    )?;
                                } else if Some(c_lower) == parsed_menu[2].1 { // Reset
                                    perform_zoom_reset(
                                        root_node,
                                        &mut app,
                                        &mut current_zoom_path,
                                        &mut layout_result,
                                        cluster_size,
                                        path,
                                        terminal,
                                    )?;
                                } else if Some(c_lower) == parsed_menu[3].1 { // Recharger
                                    if let Some(focused) = &app.focused_path {
                                        // Vérifie d'abord que c'est un dossier
                                        let is_dir = if let Some(node) = find_node_mut(root_node, std::path::Path::new(focused), std::path::Path::new(path)) {
                                            node.is_directory()
                                        } else {
                                            false
                                        };
                                        if is_dir {
                                            let _ = scanner::refresh_subfolder_cache(root_node, focused, path, None);
                                            let terminal_size = terminal.size()?;
                                            if let Some(zoom_node) = find_node_mut(root_node, &current_zoom_path, std::path::Path::new(path)) {
                                                layout_result = layout::calculate_layout(
                                                    zoom_node,
                                                    terminal_size.width as f64,
                                                    (terminal_size.height - 1) as f64,
                                                    cluster_size,
                                                    &current_zoom_path,
                                                    if app.show_free_space {
                                                        scanner::get_view_free_space(&current_zoom_path, path)
                                                    } else {
                                                        None
                                                    },
                                                    layout::Padding { header: 1.0, sides: 1.0, bottom: 1.0 },
                                                    0.001,
                                                );
                                            }
                                        }
                                    }
                                } else if is_partition && Some(c_lower) == parsed_menu[4].1 { // Alterner l'espace libre
                                    perform_toggle_free_space(
                                        root_node,
                                        &mut app,
                                        &current_zoom_path,
                                        &mut layout_result,
                                        cluster_size,
                                        path,
                                        terminal,
                                    )?;
                                } else if Some(c_lower) == parsed_menu[if is_partition { 5 } else { 4 }].1 { // Quit
                                    return Ok(());
                                }
                            }
                            _ => {}
                        },
                        AppState::InMenu => match key.code {
                            KeyCode::Esc => app.state = AppState::Browsing,
                            KeyCode::Tab | KeyCode::Right => {
                                app.selected_menu_index = (app.selected_menu_index + 1) % menu_titles.len();
                            }
                            KeyCode::Left => {
                                app.selected_menu_index = (app.selected_menu_index + menu_titles.len() - 1) % menu_titles.len();
                            }
                            KeyCode::Enter => {
                                match app.selected_menu_index {
                                    0 => { // "Zoomer"
                                        perform_zoom_in(
                                            root_node,
                                            &mut app,
                                            &mut current_zoom_path,
                                            &mut layout_result,
                                            cluster_size,
                                            path,
                                            terminal,
                                        )?;
                                        app.state = AppState::Browsing;
                                    }
                                    1 => { // "Dézoomer"
                                        perform_zoom_out(
                                            root_node,
                                            &mut app,
                                            &mut current_zoom_path,
                                            &mut layout_result,
                                            cluster_size,
                                            path,
                                            terminal,
                                        )?;
                                        app.state = AppState::Browsing;
                                    }
                                    2 => { // "Reset"
                                        perform_zoom_reset(
                                            root_node,
                                            &mut app,
                                            &mut current_zoom_path,
                                            &mut layout_result,
                                            cluster_size,
                                            path,
                                            terminal,
                                        )?;
                                        app.state = AppState::Browsing;
                                    }
                                    3 => { // "Recharger"
                                        if let Some(focused) = &app.focused_path {
                                            let is_dir = if let Some(node) = find_node_mut(root_node, std::path::Path::new(focused), std::path::Path::new(path)) {
                                                node.is_directory()
                                            } else {
                                                false
                                            };
                                            if is_dir {
                                                let _ = scanner::refresh_subfolder_cache(root_node, focused, path, None);
                                                let terminal_size = terminal.size()?;
                                                if let Some(zoom_node) = find_node_mut(root_node, &current_zoom_path, std::path::Path::new(path)) {
                                                    layout_result = layout::calculate_layout(
                                                        zoom_node,
                                                        terminal_size.width as f64,
                                                        (terminal_size.height - 1) as f64,
                                                        cluster_size,
                                                        &current_zoom_path,
                                                        if app.show_free_space {
                                                            scanner::get_view_free_space(&current_zoom_path, path)
                                                        } else {
                                                            None
                                                        },
                                                        layout::Padding { header: 1.0, sides: 1.0, bottom: 1.0 },
                                                        0.001,
                                                    );
                                                }
                                            }
                                        }
                                        app.state = AppState::Browsing;
                                    }
                                    4 if is_partition => { // "Masquer/Afficher l'espace libre"
                                        perform_toggle_free_space(
                                            root_node,
                                            &mut app,
                                            &current_zoom_path,
                                            &mut layout_result,
                                            cluster_size,
                                            path,
                                            terminal,
                                        )?;
                                        app.state = AppState::Browsing;
                                    }
                                    idx if idx == if is_partition { 5 } else { 4 } => { // "Quit"
                                        return Ok(());
                                    }
                                    _ => {}
                                }
                            }
                            KeyCode::Char(c) => {
                                let c_lower = c.to_ascii_lowercase();
                                if Some(c_lower) == parsed_menu[0].1 { // Zoomer
                                    perform_zoom_in(
                                        root_node,
                                        &mut app,
                                        &mut current_zoom_path,
                                        &mut layout_result,
                                        cluster_size,
                                        path,
                                        terminal,
                                    )?;
                                    app.state = AppState::Browsing;
                                } else if Some(c_lower) == parsed_menu[1].1 { // Dézoomer
                                    perform_zoom_out(
                                        root_node,
                                        &mut app,
                                        &mut current_zoom_path,
                                        &mut layout_result,
                                        cluster_size,
                                        path,
                                        terminal,
                                    )?;
                                    app.state = AppState::Browsing;
                                } else if Some(c_lower) == parsed_menu[2].1 { // Reset
                                    perform_zoom_reset(
                                        root_node,
                                        &mut app,
                                        &mut current_zoom_path,
                                        &mut layout_result,
                                        cluster_size,
                                        path,
                                        terminal,
                                    )?;
                                    app.state = AppState::Browsing;
                                } else if Some(c_lower) == parsed_menu[3].1 { // Recharger
                                    if let Some(focused) = &app.focused_path {
                                        let is_dir = if let Some(node) = find_node_mut(root_node, std::path::Path::new(focused), std::path::Path::new(path)) {
                                            node.is_directory()
                                        } else {
                                            false
                                        };
                                        if is_dir {
                                            let _ = scanner::refresh_subfolder_cache(root_node, focused, path, None);
                                            let terminal_size = terminal.size()?;
                                            if let Some(zoom_node) = find_node_mut(root_node, &current_zoom_path, std::path::Path::new(path)) {
                                                layout_result = layout::calculate_layout(
                                                    zoom_node,
                                                    terminal_size.width as f64,
                                                    (terminal_size.height - 1) as f64,
                                                    cluster_size,
                                                    &current_zoom_path,
                                                    if app.show_free_space {
                                                        scanner::get_view_free_space(&current_zoom_path, path)
                                                    } else {
                                                        None
                                                    },
                                                    layout::Padding { header: 1.0, sides: 1.0, bottom: 1.0 },
                                                    0.001,
                                                );
                                            }
                                        }
                                    }
                                    app.state = AppState::Browsing;
                                } else if is_partition && Some(c_lower) == parsed_menu[4].1 { // Alterner l'espace libre
                                    perform_toggle_free_space(
                                        root_node,
                                        &mut app,
                                        &current_zoom_path,
                                        &mut layout_result,
                                        cluster_size,
                                        path,
                                        terminal,
                                    )?;
                                    app.state = AppState::Browsing;
                                } else if Some(c_lower) == parsed_menu[if is_partition { 5 } else { 4 }].1 { // Quit
                                    return Ok(());
                                }
                            }
                            _ => {}
                        },
                    }
                }
            }
        }
    }
}

use diskmonger_core::layout;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::app::{App, AppState, RenderStyle};
use crate::utils::{format_size, parse_mnemonic, build_mnemonic_line};
use crate::i18n::t;

/// Fonction principale de rendu de l'interface graphique.
pub fn ui(
    frame: &mut Frame,
    app: &App,
    rects: &[layout::Rectangle],
    style: &RenderStyle,
    menu_titles: &[&str],
    menu_keys: &str,
    total_scanned_size: u64,
) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(frame.size());

    let treemap_area = main_layout[0];
    let menu_area = main_layout[1];

    frame.render_widget(Paragraph::new("").style(Style::default().bg(Color::Black)), treemap_area);

    match style {
        RenderStyle::Nested => ui_nested(frame, rects, treemap_area, &app.focused_path),
        RenderStyle::Flat => ui_flat(frame, rects, treemap_area, &app.focused_path),
    }

    draw_menu_bar(frame, app, menu_area, menu_titles, menu_keys, total_scanned_size, rects);
}

/// Rendu de l'interface en style imbriqué (Nested Treemap).
pub fn ui_nested(frame: &mut Frame, rects: &[layout::Rectangle], area: Rect, focused_path: &Option<String>) {
    let colors = [
        Color::Rgb(255, 80, 80), Color::Rgb(255, 160, 80), Color::Rgb(255, 255, 80),
        Color::Rgb(80, 255, 80), Color::Rgb(80, 255, 255), Color::Rgb(160, 160, 255),
        Color::Rgb(180, 180, 180),
    ];
    for r in rects {
        let render_area = Rect {
            x: area.x + r.x.floor() as u16, y: area.y + r.y.floor() as u16,
            width: r.width.floor() as u16, height: r.height.floor() as u16,
        };
        if render_area.width == 0 || render_area.height == 0 { continue; }
        if render_area.right() > area.right() || render_area.bottom() > area.bottom() { continue; }

        let title_text = if r.name == "other-files-name" { format!("[{}]", t("cli-other-files")) }
                         else if r.name == "free-space-name" { format!("[{}]", t("cli-free-space")) }
                         else { r.name.clone() };
        let color = colors[r.depth as usize % colors.len()];

        let is_focused = Some(&r.path) == focused_path.as_ref();
        let border_style = if is_focused {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };

        let title_span = if is_focused {
            Span::styled(
                title_text,
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            )
        } else {
            Span::raw(title_text)
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title_span);

        if is_focused {
            block = block.border_type(ratatui::widgets::BorderType::Double);
        }

        frame.render_widget(block, render_area);
    }
}

/// Rendu de l'interface en style plat (Flat Treemap).
pub fn ui_flat(frame: &mut Frame, rects: &[layout::Rectangle], area: Rect, focused_path: &Option<String>) {
    let colors = [
        Color::Rgb(80, 80, 80),
        Color::Rgb(120, 80, 80),
        Color::Rgb(80, 120, 80),
        Color::Rgb(120, 120, 80),
        Color::Rgb(80, 80, 120),
        Color::Rgb(120, 80, 120),
        Color::Rgb(80, 120, 120),
    ];
    for r in rects {
        let x_start = area.x + r.x.floor() as u16;
        let y_start = area.y + r.y.floor() as u16;
        let x_end = (x_start + r.width.floor() as u16).min(area.right());
        let y_end = (y_start + r.height.floor() as u16).min(area.bottom());

        let render_area = Rect { x: x_start, y: y_start, width: x_end - x_start, height: y_end - y_start };
        if render_area.width == 0 || render_area.height == 0 { continue; }

        let color = colors[r.depth as usize % colors.len()];
        let block = Paragraph::new("").style(Style::default().bg(color));
        frame.render_widget(block, render_area);

        if render_area.width > 2 && render_area.height > 0 {
            let mut title_text = if r.name == "other-files-name" { format!("[{}]", t("cli-other-files")) }
                             else if r.name == "free-space-name" { format!("[{}]", t("cli-free-space")) }
                             else { r.name.clone() };

            let is_focused = Some(&r.path) == focused_path.as_ref();
            if is_focused {
                title_text = format!("* {}", title_text);
            }

            let title_style = if is_focused {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD).add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };

            let title_line = Line::raw(title_text).patch_style(title_style);
            let text_area = Rect { x: render_area.x + 1, y: render_area.y, width: render_area.width.saturating_sub(1), height: 1 };

            frame.render_widget(Paragraph::new(title_line).style(Style::default().bg(color)), text_area);
        }
    }
}

/// Rendu de la barre d'onglets de menu inférieure et des informations sur l'élément focus.
pub fn draw_menu_bar(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    titles: &[&str],
    keys: &str,
    total_scanned_size: u64,
    rects: &[layout::Rectangle],
) {
    let shortcut_text = t("cli-shortcut-hint");

    // Détermine le texte d'information de l'élément sélectionné
    let mut focus_text = String::new();
    if let Some(focused) = &app.focused_path {
        if let Some(r) = rects.iter().find(|rect| &rect.path == focused) {
            let formatted_size = format_size(r.size);
            let percentage = if total_scanned_size > 0 {
                (r.size as f64 / total_scanned_size as f64) * 100.0
            } else {
                0.0
            };
            let display_name = if r.name == "other-files-name" {
                t("cli-other-files")
            } else if r.name == "free-space-name" {
                t("cli-free-space")
            } else {
                r.name.clone()
            };
            focus_text = format!("{} ({} - {:.2}%)", display_name, formatted_size, percentage);
        }
    }

    let (keys_clean, keys_underline_idx, _) = parse_mnemonic(keys);

    // Calcul de la largeur requise de manière dynamique pour s'adapter à toutes les langues (ex: Allemand)
    let total_titles_len: usize = titles.iter().map(|&t| {
        let (clean, _, _) = parse_mnemonic(t);
        clean.chars().count()
    }).sum();
    let dividers_count = if titles.is_empty() { 0 } else { titles.len() - 1 };
    let tabs_width = total_titles_len + dividers_count * 3 + 2; // +2 pour la sécurité de marge

    let menu_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(keys_clean.chars().count() as u16 + 1), // keys + 1 pour l'espace de séparation
            Constraint::Length(tabs_width as u16), // Largeur dynamique calculée pour s'adapter à toutes les langues
            Constraint::Min(0),      // Zone du milieu dynamique pour les infos du focus
            Constraint::Length(shortcut_text.chars().count() as u16)
        ].as_ref())
        .split(area);

    let keys_area = menu_layout[0];
    let tabs_area = menu_layout[1];
    let focus_info_area = menu_layout[2];
    let shortcuts_area = menu_layout[3];

    let keys_line = {
        let mut line = build_mnemonic_line(&keys_clean, keys_underline_idx);
        for span in &mut line.spans {
            span.style = span.style.fg(Color::White).add_modifier(Modifier::BOLD);
        }
        line.spans.push(Span::raw(" "));
        line
    };
    frame.render_widget(Paragraph::new(keys_line), keys_area);

    let titles_lines: Vec<Line> = titles.iter().cloned().map(|t| {
        let (clean_text, underline_idx, _) = parse_mnemonic(t);
        build_mnemonic_line(&clean_text, underline_idx)
    }).collect();

    let highlight_style = if matches!(app.state, AppState::InMenu) {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
            .bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Gray)
    };

    let tabs = Tabs::new(titles_lines)
        .select(app.selected_menu_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(highlight_style);
    frame.render_widget(tabs, tabs_area);

    // Dessine l'information du focus au milieu (centrée)
    let focus_span = Span::styled(focus_text, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(Paragraph::new(focus_span).alignment(Alignment::Center), focus_info_area);

    let shortcuts_span = Span::styled(shortcut_text, Style::default().fg(Color::DarkGray));
    frame.render_widget(Paragraph::new(shortcuts_span), shortcuts_area);
}

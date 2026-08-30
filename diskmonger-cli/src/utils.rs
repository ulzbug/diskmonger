use diskmonger_core::{layout, scanner};
use ratatui::prelude::*;

/// Analyse une chaîne contenant un marqueur de mnémonique `&`.
/// Retourne un tuple avec :
/// 1. La chaîne de caractères propre (sans le marqueur `&`).
/// 2. L'index de la lettre à souligner (si trouvé).
/// 3. Le caractère de raccourci en minuscule (si trouvé).
pub fn parse_mnemonic(translated_str: &str) -> (String, Option<usize>, Option<char>) {
    let mut clean_str = String::new();
    let mut underline_idx = None;
    let mut shortcut_char = None;
    let mut chars = translated_str.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' && underline_idx.is_none() {
            if let Some(&next_c) = chars.peek() {
                underline_idx = Some(clean_str.chars().count());
                shortcut_char = Some(next_c.to_ascii_lowercase());
                continue; // Saute l'esperluette
            }
        }
        clean_str.push(c);
    }
    (clean_str, underline_idx, shortcut_char)
}

/// Construit un `Line` ratatui à partir d'un texte et de l'index de la lettre à souligner.
pub fn build_mnemonic_line(text: &str, underline_idx: Option<usize>) -> Line<'static> {
    if let Some(idx) = underline_idx {
        let chars: Vec<char> = text.chars().collect();
        let before: String = chars[..idx].iter().collect();
        let under = chars[idx].to_string();
        let after: String = chars[idx + 1..].iter().collect();
        Line::from(vec![
            Span::raw(before),
            Span::styled(under, Style::default().add_modifier(Modifier::UNDERLINED)),
            Span::raw(after),
        ])
    } else {
        Line::from(text.to_string())
    }
}

/// Formate une taille en octets en une chaîne lisible par l'homme (B, KB, MB, GB, etc.).
pub fn format_size(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < units.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.2} {}", size, units[unit_idx])
}

/// Recherche de manière récursive et efficace un nœud mutable (`FsNode`) dans l'arbre à partir de son chemin absolu.
pub use scanner::find_node_mut;

/// Récupère les chemins des éléments frères (siblings) visibles à l'écran pour un chemin donné.
pub fn get_visible_siblings(
    target_path_str: &str,
    rects: &[layout::Rectangle],
) -> Option<Vec<String>> {
    let target_path = std::path::Path::new(target_path_str);
    let parent_path = target_path.parent()?;

    let siblings: Vec<String> = rects.iter()
        .filter(|r| {
            if let Some(r_parent) = std::path::Path::new(&r.path).parent() {
                r_parent == parent_path
            } else {
                false
            }
        })
        .map(|r| r.path.clone())
        .collect();

    if siblings.is_empty() {
        None
    } else {
        Some(siblings)
    }
}

/// Récupère le chemin absolu du premier enfant visible à l'écran d'un dossier donné.
pub fn get_first_visible_child_path(
    target_path_str: &str,
    rects: &[layout::Rectangle],
) -> Option<String> {
    let target_path = std::path::Path::new(target_path_str);

    let first_child = rects.iter().find(|r| {
        if let Some(r_parent) = std::path::Path::new(&r.path).parent() {
            r_parent == target_path
        } else {
            false
        }
    })?;

    Some(first_child.path.clone())
}

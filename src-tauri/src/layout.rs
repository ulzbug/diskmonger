//! Ce module utilise la crate 'treemap' pour calculer la mise en page.

use crate::scanner::FsEntry;
use treemap::{Rect, Mappable, TreemapLayout};

#[derive(Debug, serde::Serialize, Clone)]
pub struct Rectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub depth: u32,
    pub path: String,
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
}

impl Mappable for FsEntry {
    fn size(&self) -> f64 {
        self.size as f64
    }
    fn bounds(&self) -> &Rect {
        &self.bounds
    }
    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }
}

fn generate_treemap_rects(
    rectangles: &mut Vec<Rectangle>,
    items: &mut [FsEntry],
    bounds: Rect,
    depth: u32,
) {
    if items.is_empty() { return; }

    let layout = TreemapLayout::new();
    layout.layout_items(items, bounds);
    
    for item in items {
        let item_bounds = item.bounds();
        rectangles.push(Rectangle {
            x: item_bounds.x,
            y: item_bounds.y,
            width: item_bounds.w,
            height: item_bounds.h,
            depth,
            path: item.path.clone(),
            name: item.name.clone(),
            is_directory: item.is_directory,
            size: item.size,
        });

        if item.is_directory && !item.children.is_empty() {
            const HEADER_HEIGHT: f64 = 12.0; // En-tête pour le nom du dossier
            const SIDE_PADDING: f64 = 4.0;   // Marge sur les côtés
            const BOTTOM_PADDING: f64 = 4.0; // Marge en bas

            if item_bounds.w > (SIDE_PADDING * 2.0) && item_bounds.h > (HEADER_HEIGHT + BOTTOM_PADDING) {
                let inner_bounds = Rect::from_points(
                    item_bounds.x + SIDE_PADDING,
                    item_bounds.y + HEADER_HEIGHT,
                    item_bounds.w - (SIDE_PADDING * 2.0),
                    item_bounds.h - HEADER_HEIGHT - BOTTOM_PADDING,
                );
                if inner_bounds.w > 1.0 && inner_bounds.h > 1.0 {
                    generate_treemap_rects(rectangles, &mut item.children, inner_bounds, depth + 1);
                }
            }
        }
    }
}

pub fn calculate_layout(root: &mut FsEntry, width: f64, height: f64) -> Vec<Rectangle> {
    let mut rectangles = Vec::new();
    if root.is_directory {
        let initial_bounds = Rect::from_points(0.0, 0.0, width, height);
        generate_treemap_rects(&mut rectangles, &mut root.children, initial_bounds, 0);
    }
    rectangles
}

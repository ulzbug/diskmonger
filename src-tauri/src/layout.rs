//! Calcule la disposition en treemap et génère les rectangles pour le rendu.
use crate::scanner::{FsNode, FsFile};
use std::path::Path;
use treemap::{Rect, Mappable, TreemapLayout};

/// Représente un rectangle dessiné à l'écran.
/// Contient les coordonnées, dimensions, et métadonnées du noeud de système de fichiers correspondant.
#[derive(Debug, serde::Serialize, Clone)]
pub struct Rectangle {
    pub x: f64, pub y: f64, pub width: f64, pub height: f64,
    pub depth: u32, pub path: String, pub name: String,
    pub is_directory: bool, pub size: u64,
}

/// Conteneur pour le résultat complet d'un calcul de layout.
/// Comprend les rectangles à dessiner, ainsi que les statistiques globales de la vue.
#[derive(Debug, serde::Serialize, Clone)]
pub struct LayoutResult {
    pub rectangles: Vec<Rectangle>,
    pub total_items: usize,
    pub total_size: u64,
}

/// Structure interne utilisée par la bibliothèque `treemap` pour mapper les données.
struct LayoutEntry<'a> {
    node: &'a mut FsNode,
    bounds: Rect,
}

impl<'a> Mappable for LayoutEntry<'a> {
    fn size(&self) -> f64 { self.node.size_in_clusters() as f64 }
    fn bounds(&self) -> &Rect { &self.bounds }
    fn set_bounds(&mut self, bounds: Rect) { self.bounds = bounds; }
}

/// Seuil de pixels en dessous duquel un rectangle n'est pas dessiné.
const MIN_PIXEL_THRESHOLD: f64 = 5.0;

/// Génère récursivement la liste plate de tous les rectangles visibles pour une vue donnée.
///
/// # Arguments
/// * `rectangles` - Le vecteur mutable où les rectangles générés sont ajoutés.
/// * `nodes` - La liste des noeuds (fichiers/dossiers) à disposer.
/// * `bounds` - Le rectangle parent dans lequel les noeuds doivent être disposés.
/// * `depth` - La profondeur de récursion actuelle (pour la couleur et le décalage).
/// * `base_path` - Le chemin absolu du parent, utilisé pour construire les chemins des enfants.
/// * `threshold` - Le seuil de taille (en clusters) en dessous duquel les éléments sont regroupés.
/// * `cluster_size` - La taille d'un cluster en octets, pour calculer la taille finale.
fn generate_treemap_rects(
    rectangles: &mut Vec<Rectangle>,
    nodes: &mut [FsNode],
    bounds: Rect,
    depth: u32,
    base_path: &Path,
    threshold: u32,
    cluster_size: u64,
) {
    if nodes.is_empty() || bounds.w < 1.0 || bounds.h < 1.0 { return; }

    // --- 1. Filtrage Pré-Layout ---
    // Regroupe tous les éléments (fichiers et dossiers) plus petits que le seuil
    // dans un noeud virtuel "Autres fichiers" pour éviter de polluer l'affichage.
    let mut display_nodes: Vec<FsNode> = Vec::new();
    let mut small_items_size_in_clusters: u32 = 0;

    for node in nodes.iter_mut() {
        if node.size_in_clusters() < threshold {
            small_items_size_in_clusters = small_items_size_in_clusters.saturating_add(node.size_in_clusters());
        } else {
            display_nodes.push(node.clone());
        }
    }

    if small_items_size_in_clusters > 0 {
        display_nodes.push(FsNode::File(FsFile {
            name: "other-files-name".into(),
            size_in_clusters: small_items_size_in_clusters,
        }));
    }

    // --- 2. Calcul du Layout ---
    let mut layout_entries: Vec<LayoutEntry> = display_nodes.iter_mut().map(|node| {
        LayoutEntry { node, bounds: Rect::new() }
    }).collect();

    let layout_engine = TreemapLayout::new();
    layout_engine.layout_items(&mut layout_entries, bounds);

    // --- 3. Génération des Rectangles et Récursion ---
    for mut entry in layout_entries {
        if entry.bounds.w < MIN_PIXEL_THRESHOLD || entry.bounds.h < MIN_PIXEL_THRESHOLD {
            continue;
        }

        let item_path = base_path.join(entry.node.name());
        rectangles.push(Rectangle {
            x: entry.bounds.x, y: entry.bounds.y,
            width: entry.bounds.w, height: entry.bounds.h,
            depth,
            path: item_path.to_string_lossy().to_string(),
            name: entry.node.name().to_string(),
            is_directory: entry.node.is_directory(),
            size: entry.node.size_in_clusters() as u64 * cluster_size,
        });
        
        // Si c'est un dossier avec des enfants, on descend récursivement.
        if let FsNode::Dir(dir) = &mut entry.node {
            if !dir.children.is_empty() {
                const HEADER_HEIGHT: f64 = 12.0;
                const SIDE_PADDING: f64 = 4.0;
                const BOTTOM_PADDING: f64 = 4.0;

                if entry.bounds.w > (SIDE_PADDING * 2.0) && entry.bounds.h > (HEADER_HEIGHT + BOTTOM_PADDING) {
                    let inner_bounds = Rect::from_points(
                        entry.bounds.x + SIDE_PADDING,
                        entry.bounds.y + HEADER_HEIGHT,
                        entry.bounds.w - (SIDE_PADDING * 2.0),
                        entry.bounds.h - HEADER_HEIGHT - BOTTOM_PADDING,
                    );
                    if inner_bounds.w > 1.0 && inner_bounds.h > 1.0 {
                        generate_treemap_rects(rectangles, &mut dir.children, inner_bounds, depth + 1, &item_path, threshold, cluster_size);
                    }
                }
            }
        }
    }
}

/// Point d'entrée principal pour calculer le layout d'une vue.
/// Calcule le seuil, les statistiques, et lance la génération récursive des rectangles.
pub fn calculate_layout(root: &mut FsNode, width: f64, height: f64, cluster_size: u64, view_path: &std::path::Path) -> LayoutResult {
    let mut rectangles = Vec::new();
    let total_items = root.count_items().saturating_sub(1);
    
    if let FsNode::Dir(dir) = root {
        let initial_bounds = Rect::from_points(0.0, 0.0, width, height);
        // Le seuil de regroupement est de 0.02% de la taille totale de la vue actuelle.
        let threshold = (dir.size_in_clusters as f64 * 0.0002) as u32;
        let total_size = dir.size_in_clusters as u64 * cluster_size;

        generate_treemap_rects(&mut rectangles, &mut dir.children, initial_bounds, 0, view_path, threshold, cluster_size);
        
        LayoutResult { rectangles, total_items, total_size }
    } else {
        // Cas d'un fichier à la racine (ne devrait pas arriver en pratique pour une vue).
        LayoutResult { rectangles, total_items: 1, total_size: root.size_in_clusters() as u64 * cluster_size }
    }
}

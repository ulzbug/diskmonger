// Ce fichier est le point d'entrée principal de l'interface utilisateur.
// Il gère l'état global, les interactions utilisateur, la communication avec le backend Rust,
// et le rendu du treemap sur le canvas HTML5.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { confirm } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";

// --- ÉTAT GLOBAL DE L'APPLICATION ---
const appWindow = getCurrentWindow();
let scanPathInputEl: HTMLInputElement | null;
let treemapCanvasEl: HTMLCanvasElement | null;
let treemapCtx: CanvasRenderingContext2D | null = null;
let treemapTooltipEl: HTMLElement | null = null;

let currentRectangles: Rectangle[] = []; // Les rectangles actuellement affichés
let scanRootPath: string | null = null; // Le chemin absolu du scan initial
let currentPathSegments: string[] = []; // Les segments du chemin de la vue actuelle, ex: ['Documents', 'Projets']
let selectedRectangle: Rectangle | null = null; // Le rectangle actuellement sélectionné par un clic

let i18nMessages: Record<string, string> = {}; // Cache pour les messages de traduction

// --- INTERFACES (Contrats de données avec Rust) ---
interface Rectangle {
    x: number; y: number; width: number; height: number;
    depth: number; path: string; name: string;
    is_directory: boolean; size: number;
}

interface LayoutResult {
    rectangles: Rectangle[];
    total_items: number;
    total_size: number;
}

// --- INTERNATIONALISATION (i18n) ---
/** Traduit une clé en utilisant le cache `i18nMessages` chargé. */
function tr(key: string, args?: Record<string, string | number>): string {
    let message = i18nMessages[key];
    if (!message) return key; // Retourne la clé si non trouvée
    if (args) {
        for (const [argKey, argValue] of Object.entries(args)) {
            message = message.replace(`{${argKey}}`, String(argValue));
        }
    }
    return message;
}

// --- FONCTIONS UTILITAIRES ---
/** Formate un nombre d'octets en une chaîne lisible (Ko, Mo, Go...). */
function formatBytes(bytes: number, decimals = 2): string {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}


// --- GESTION DU RENDU DU CANVAS ---
const COLORS = ['#FF7F7F', '#FFBF7F', '#FFFF00', '#7FFF7F', '#7FFFFF', '#BFBFFF', '#BFBFBF', '#FF7FFF'];

/** Redessine l'intégralité du treemap sur le canvas à partir de `currentRectangles`. */
async function renderTreemap() {
    if (!treemapCanvasEl || !treemapCtx) return;
    const { width, height } = treemapCanvasEl.getBoundingClientRect();
    treemapCanvasEl.width = width;
    treemapCanvasEl.height = height;
    treemapCtx.clearRect(0, 0, width, height);

    for (const rect of currentRectangles) {
        if (!rect) continue; // Sécurité

        // Dessine le fond du rectangle avec une couleur basée sur sa profondeur
        treemapCtx.fillStyle = COLORS[rect.depth % COLORS.length];
        treemapCtx.fillRect(rect.x, rect.y, rect.width, rect.height);

        // Dessine la bordure (blanche si sélectionné, noire sinon)
        treemapCtx.strokeStyle = (selectedRectangle && rect.path === selectedRectangle.path) ? '#FFFFFF' : '#000000';
        treemapCtx.lineWidth = (selectedRectangle && rect.path === selectedRectangle.path) ? 2 : 1;
        treemapCtx.strokeRect(rect.x, rect.y, rect.width, rect.height);

        // N'affiche le texte que si le rectangle est assez large
        if (rect.width >= 25) {
            const displayName = tr(rect.name);
            if (rect.name == 'other-files-name') {
                // Ne rien dessiner pour les groupes "[Autres...]"
            } else if (rect.is_directory) {
                treemapCtx.fillStyle = '#000000';
                treemapCtx.font = "8px sans-serif";
                treemapCtx.fillText(displayName, rect.x + 3, rect.y + 10);
            } else {
                treemapCtx.fillStyle = '#000000';
                treemapCtx.font = "8px sans-serif";
                treemapCtx.fillText(displayName, rect.x + 4, rect.y + 10);
            }
        }
    }
}

/** Affiche un message (ex: "Scan en cours...") au centre du canvas. */
async function showScanMessage(textKey: string, subtext: string = "", files: number = 0, dirs: number = 0) {
    if (!treemapCtx || !treemapCanvasEl) return;
    const { width, height } = treemapCanvasEl.getBoundingClientRect();
    treemapCanvasEl.width = width;
    treemapCanvasEl.height = height;
    treemapCtx.clearRect(0, 0, width, height);
    treemapCtx.fillStyle = '#FFFFFF';

    treemapCtx.font = "18px sans-serif";
    treemapCtx.fillText(tr(textKey), 50, 50);

    if (subtext) {
        treemapCtx.font = "14px sans-serif";
        treemapCtx.fillText(subtext, 50, 80);
    }
    if (files > 0 || dirs > 0) {
        const numberFormatter = new Intl.NumberFormat();
        const stats = tr('scan-stats', { files: numberFormatter.format(files), dirs: numberFormatter.format(dirs) });
        treemapCtx.font = "14px sans-serif";
        treemapCtx.fillText(stats, 50, 110);
    }
}

/** Cache l'infobulle, désélectionne le rectangle et redessine le canvas. */
function hideTooltipAndDeselect() {
    if (treemapTooltipEl) treemapTooltipEl.style.display = 'none';
    if (selectedRectangle) {
        selectedRectangle = null;
        renderTreemap();
    }
}


// --- GESTIONNAIRES D'ÉVÉNEMENTS (Logique principale) ---

/** Lance un nouveau scan du chemin spécifié dans l'input. */
async function performScan() {
    if (scanPathInputEl && treemapCanvasEl) {
        const path = scanPathInputEl.value;
        scanRootPath = path;
        hideTooltipAndDeselect();
        await showScanMessage("scanning-title", path);
        await appWindow.setTitle(tr('window-title-scanning'));
        invoke("scan", { path });
    }
}

/**
 * Fonction centrale pour le zoom. Appelle le backend pour obtenir le nouveau layout
 * et met à jour l'état de l'application (rectangles, titre, état des boutons).
 */
async function zoomIn(segments: string[]) {
    if (!treemapCanvasEl) return;
    try {
        hideTooltipAndDeselect();
        currentPathSegments = segments;
        const { width, height = 0 } = treemapCanvasEl.getBoundingClientRect();
        const result = await invoke<LayoutResult>("zoom_in", { segments, width, height });
        currentRectangles = result.rectangles;

        const currentFullPath = [scanRootPath, ...segments].join('/') || scanRootPath;
        await appWindow.setTitle(tr('window-title-viewing', {
            path: currentFullPath,
            size: formatBytes(result.total_size),
            items: result.total_items.toString()
        }));

        updateButtonStates();
        await renderTreemap();
    } catch (error) {
        console.error("Zoom failed:", error);
        await showScanMessage("scan-failed-title", `${error}`);
    }
}

/** Gère le clic sur le bouton "Dézoomer". */
async function zoomOut() {
    if (currentPathSegments.length > 0) {
        const newSegments = [...currentPathSegments];
        newSegments.pop();
        await zoomIn(newSegments);
    }
}

/** Gère le clic sur le bouton "Réinitialiser". */
function resetZoom() {
    if (currentPathSegments.length > 0) {
        zoomIn([]);
    }
}

/** Ouvre la boîte de dialogue native pour choisir un dossier. */
async function browse() {
    const result = await open({
        directory: true, multiple: false,
        defaultPath: [scanRootPath, ...currentPathSegments].join('/')
    });
    if (typeof result === 'string' && scanPathInputEl) {
        scanPathInputEl.value = result;
        await performScan();
    }
}

/** Gère le clic simple sur le canvas (sélection et affichage de l'infobulle). */
async function handleCanvasClick(event: MouseEvent) {
    const clickedRect = getClickedRectangle(event);

    if (!clickedRect) {
        hideTooltipAndDeselect();
        return;
    }
    if (selectedRectangle && selectedRectangle.path === clickedRect.path) {
        hideTooltipAndDeselect();
        return;
    }

    selectedRectangle = clickedRect;
    await renderTreemap();

    if (treemapTooltipEl && treemapCanvasEl) {
        treemapTooltipEl.innerHTML = '';
        const sizeStr = formatBytes(selectedRectangle.size);
        const typeKey = selectedRectangle.is_directory ? 'tooltip-type-dir' : 'tooltip-type-file';
        const totalDisplayedSize = currentRectangles.reduce((sum, r) => r.depth === 0 ? sum + r.size : sum, 0);
        const percentage = totalDisplayedSize > 0 ? (selectedRectangle.size / totalDisplayedSize) * 100 : 0;

        // Rétablir les bonnes informations détaillées de l'ancienne version
        treemapTooltipEl.innerHTML = `
            <strong>${tr(selectedRectangle.name)}</strong>
            <div class="tooltip-info"><span>${tr('tooltip-size-label')}:</span> ${sizeStr} (${percentage.toFixed(2)}%)</div>
            <div class="tooltip-info"><span>${tr('tooltip-type-label')}:</span> ${tr(typeKey)}</div>
            <div class="tooltip-info"><span>${tr('tooltip-path-label')}:</span> ${selectedRectangle.path}</div>
            <hr>
        `;

        const actionsEl = document.createElement('div');
        actionsEl.className = 'tooltip-actions';

        const openBtn = document.createElement('button');
        openBtn.textContent = selectedRectangle.is_directory ? tr('tooltip-action-zoom') : tr('tooltip-action-open');
        openBtn.onclick = () => {
            if (selectedRectangle) {
                if (selectedRectangle.is_directory) {
                    handleCanvasDblClick(event);
                } else {
                    invoke('tauri-plugin-opener:open', { path: selectedRectangle.path });
                }
            }
        };
        actionsEl.appendChild(openBtn);

        const revealBtn = document.createElement('button');
        revealBtn.textContent = tr('tooltip-action-reveal');
        revealBtn.onclick = () => {
            if (selectedRectangle) invoke('reveal_in_explorer', { path: selectedRectangle.path });
        };
        actionsEl.appendChild(revealBtn);

        const copyBtn = document.createElement('button');
        copyBtn.textContent = tr('tooltip-action-copy-path');
        copyBtn.onclick = () => {
            if (selectedRectangle) navigator.clipboard.writeText(selectedRectangle.path);
        };
        actionsEl.appendChild(copyBtn);

        const deleteBtn = document.createElement('button');
        deleteBtn.className = 'danger';
        deleteBtn.textContent = tr('tooltip-action-delete');
        deleteBtn.onclick = async () => {
            if (selectedRectangle) {
                // Utiliser la fonction de confirmation native confirm()
                const confirmed = await confirm(tr('delete-confirm-message', { path: selectedRectangle.path }), {
                    title: tr('delete-confirm-title'),
                    kind: 'warning',
                    okLabel: tr('delete-confirm-ok'),
                    cancelLabel: tr('delete-confirm-cancel'),
                });
                if (confirmed) {
                    invoke('trash_item', { path: selectedRectangle.path })
                        .then(() => {
                            hideTooltipAndDeselect();
                            zoomIn(currentPathSegments);
                        })
                        .catch(e => console.error("Failed to delete item:", e));
                }
            }
        };
        actionsEl.appendChild(deleteBtn);

        treemapTooltipEl.appendChild(actionsEl);

        treemapTooltipEl.style.display = 'block';
        treemapTooltipEl.style.pointerEvents = 'auto';

        // Calculer les dimensions de l'infobulle pour le positionnement intelligent
        const tooltipRect = treemapTooltipEl.getBoundingClientRect();
        const canvasRect = treemapCanvasEl.getBoundingClientRect();
        let top = event.clientY + 10;
        let left = event.clientX + 10;

        // Positionnement intelligent vers le haut si ça déborde en bas
        if (top + tooltipRect.height > window.innerHeight) {
            top = event.clientY - tooltipRect.height - 10;
        }
        // Positionnement intelligent vers la gauche si ça déborde à droite
        if (left + tooltipRect.width > window.innerWidth) {
            left = event.clientX - tooltipRect.width - 10;
        }

        treemapTooltipEl.style.left = `${left - canvasRect.left}px`;
        treemapTooltipEl.style.top = `${top - canvasRect.top}px`;
    }
}

/** Gère le double-clic sur le canvas (zoom dans un dossier). */
function handleCanvasDblClick(event: MouseEvent) {
    const clickedRect = getClickedRectangle(event);
    if (!clickedRect || !clickedRect.is_directory || !scanRootPath) return;
    if (clickedRect.name.startsWith('[')) return;
    if (clickedRect.path === scanRootPath) {
        zoomIn([]);
        return;
    }

    if (clickedRect.path.startsWith(scanRootPath)) {
        let relativePath = clickedRect.path.substring(scanRootPath.length);
        if (relativePath.startsWith('/')) {
            relativePath = relativePath.substring(1);
        }
        const newSegments = relativePath.split('/').filter(s => s.length > 0 && s !== '.');
        zoomIn(newSegments);
    }
}

/** Trouve le rectangle le plus spécifique (le plus petit en surface) sous le curseur. */
function getClickedRectangle(event: MouseEvent): Rectangle | null {
    if (!treemapCanvasEl) return null;
    const x = event.offsetX;
    const y = event.offsetY;
    let clickedRect: Rectangle | null = null;
    for (const r of currentRectangles) {
        if (!r) continue;
        if (x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height) {
            // On garde le plus petit rectangle (le plus "profond")
            if (!clickedRect || (r.width * r.height < clickedRect.width * clickedRect.height)) {
                clickedRect = r;
            }
        }
    }
    return clickedRect;
}

/** Met à jour l'état activé/désactivé des boutons de zoom. */
function updateButtonStates() {
    const zoomOutBtn = document.querySelector("#zoom-out-btn") as HTMLButtonElement | null;
    const resetZoomBtn = document.querySelector("#reset-zoom-btn") as HTMLButtonElement | null;

    const isAtRoot = currentPathSegments.length === 0;

    if (zoomOutBtn) zoomOutBtn.disabled = isAtRoot;
    if (resetZoomBtn) resetZoomBtn.disabled = isAtRoot;
}


// --- INITIALISATION AU CHARGEMENT DE LA PAGE ---
window.addEventListener("DOMContentLoaded", async () => {
    // Récupération des éléments du DOM
    scanPathInputEl = document.querySelector("#scan-path-input");
    treemapCanvasEl = document.querySelector("#treemap-canvas");
    treemapTooltipEl = document.querySelector("#treemap-tooltip");
    if (treemapCanvasEl) {
        treemapCtx = treemapCanvasEl.getContext('2d');
        treemapCanvasEl.addEventListener('click', handleCanvasClick);
        treemapCanvasEl.addEventListener('dblclick', handleCanvasDblClick);
    }

    // Écouteurs d'événements globaux
    window.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') hideTooltipAndDeselect();
    });
    document.querySelector(".toolbar")?.addEventListener('click', (e) => {
        if (e.target && (e.target as HTMLElement).tagName !== 'INPUT') {
            hideTooltipAndDeselect();
        }
    });
    document.querySelector("#scan-form")?.addEventListener("submit", (e) => { e.preventDefault(); performScan(); });
    document.querySelector("#browse-btn")?.addEventListener("click", browse);
    document.querySelector("#zoom-out-btn")?.addEventListener("click", zoomOut);
    document.querySelector("#reset-zoom-btn")?.addEventListener("click", resetZoom);
    window.addEventListener('resize', () => {
        if (scanRootPath && currentRectangles.length > 0) {
            zoomIn(currentPathSegments);
        }
    });

    // Écouteurs pour les événements du backend Rust
    listen('scan-progress', async (event) => {
        const payload = event.payload as { path: string, files: number, dirs: number };
        await showScanMessage("scanning-title", payload.path, payload.files, payload.dirs);
        await appWindow.setTitle(tr('window-title-scanning'));
    });
    listen('scan-complete', async () => {
        await zoomIn([]); // Affiche la racine après un scan
    });
    listen('scan-error', async (event) => {
        console.error("Scan failed:", event.payload);
        await showScanMessage("scan-failed-title", `${event.payload}`);
    });

    // Chargement des traductions et initialisation de l'UI
    try {
        let locale = await invoke<string>("get_locale");
        const response = await fetch(`/locales/${locale}.json`);
        if (!response.ok) { // Si la traduction n'existe pas, on bascule sur l'anglais
            locale = 'en';
            const fallbackResponse = await fetch(`/locales/en.json`);
            i18nMessages = await fallbackResponse.json();
        } else {
            i18nMessages = await response.json();
        }
    } catch (e) { console.error("Failed to load translations", e); }

    document.querySelector('#path-label')!.textContent = tr('path-label');
    document.querySelector('#browse-btn')!.textContent = tr('browse-btn');
    document.querySelector('#scan-btn')!.textContent = tr('scan-btn');
    document.querySelector('#zoom-out-btn')!.textContent = tr('zoom-out-btn');
    document.querySelector('#reset-zoom-btn')!.textContent = tr('reset-btn');
    scanPathInputEl!.placeholder = tr('path-input-placeholder');
    await appWindow.setTitle(tr('window-title-default'));

    try {
        const defaultPath = await invoke<string>("get_default_scan_path");
        if (scanPathInputEl) {
            scanPathInputEl.value = defaultPath;
            scanRootPath = defaultPath;
        }
    } catch (error) {
        console.error("Failed to get default path:", error);
    }

    updateButtonStates(); // État initial des boutons
});

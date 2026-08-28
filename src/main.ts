import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";

// --- GLOBAL STATE & SETUP ---
const appWindow = getCurrentWindow();
let scanPathInputEl: HTMLInputElement | null;
let treemapCanvasEl: HTMLCanvasElement | null;
let treemapCtx: CanvasRenderingContext2D | null = null;
let treemapTooltipEl: HTMLElement | null = null;

let currentRectangles: Rectangle[] = [];
let scanRootPath: string | null = null;
let currentPathSegments: string[] = [];
let selectedRectangle: Rectangle | null = null;

let i18nMessages: Record<string, string> = {};

// --- INTERFACES ---
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

// --- i18n HELPER ---
function tr(key: string, args?: Record<string, string | number>): string {
    let message = i18nMessages[key];
    if (!message) return key;

    if (args) {
        for (const [argKey, argValue] of Object.entries(args)) {
            message = message.replace(`{${argKey}}`, String(argValue));
        }
    }
    return message;
}

// --- HELPERS ---
function formatBytes(bytes: number, decimals = 2): string {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}


// --- UI RENDERING ---
const COLORS = ['#FF7F7F', '#FFBF7F', '#FFFF00', '#7FFF7F', '#7FFFFF', '#BFBFFF', '#BFBFBF', '#FF7FFF'];

async function renderTreemap() {
    if (!treemapCanvasEl || !treemapCtx) return;
    const { width, height } = treemapCanvasEl.getBoundingClientRect();
    treemapCanvasEl.width = width;
    treemapCanvasEl.height = height;
    treemapCtx.clearRect(0, 0, width, height);

    for (const rect of currentRectangles) {
        if (!rect) continue; // Garde-fou

        treemapCtx.fillStyle = COLORS[rect.depth % COLORS.length];
        treemapCtx.fillRect(rect.x, rect.y, rect.width, rect.height);

        treemapCtx.strokeStyle = (selectedRectangle && rect.path === selectedRectangle.path) ? '#FFFFFF' : '#000000';
        treemapCtx.lineWidth = (selectedRectangle && rect.path === selectedRectangle.path) ? 2 : 1;
        treemapCtx.strokeRect(rect.x, rect.y, rect.width, rect.height);

        if (rect.width >= 25) {
            const displayName = rect.name === 'other-files-name' ? tr(rect.name) : rect.name;
            if (rect.name === 'other-files-name') {
                // Ne rien dessiner pour les "Autres fichiers", laisser le rectangle vide
            } else if (rect.is_directory) {
                treemapCtx.fillStyle = '#000000';
                treemapCtx.font = "9px sans-serif"; // normal
                treemapCtx.fillText(displayName, rect.x + 3, rect.y + 10);
            } else {
                treemapCtx.fillStyle = '#000000';
                treemapCtx.font = "8px sans-serif";
                treemapCtx.fillText(displayName, rect.x + 4, rect.y + 10);
            }
        }
    }
}

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


// --- EVENT HANDLERS ---
async function performScan() {
    if (scanPathInputEl && treemapCanvasEl) {
        const path = scanPathInputEl.value;
        scanRootPath = path;
        currentPathSegments = [];
        selectedRectangle = null;
        if (treemapTooltipEl) treemapTooltipEl.style.display = 'none';

        await showScanMessage("scanning-title", path);
        await appWindow.setTitle(tr('window-title-scanning'));
        invoke("scan", { path });
    }
}

async function zoomIn(segments: string[]) {
    console.log(`[zoomIn] Invoking zoom_in with segments:`, segments);
    if (!treemapCanvasEl) return;
    try {
        selectedRectangle = null;
        if (treemapTooltipEl) treemapTooltipEl.style.display = 'none';

        currentPathSegments = segments;
        const { width, height = 0 } = treemapCanvasEl.getBoundingClientRect();
        const result = await invoke<LayoutResult>("zoom_in", { segments, width, height });
        currentRectangles = result.rectangles;

        const currentFullPath = [scanRootPath, ...segments].join('/');
            
        await appWindow.setTitle(tr('window-title-viewing', {
            path: currentFullPath,
            size: formatBytes(result.total_size),
            items: result.total_items.toString()
        }));

        await renderTreemap();
    } catch (error) {
        console.error("Zoom failed:", error);
        await showScanMessage("scan-failed-title", `${error}`);
    }
}

async function zoomOut() {
    if (currentPathSegments.length === 0) return;
    try {
        selectedRectangle = null;
        if (treemapTooltipEl) treemapTooltipEl.style.display = 'none';

        currentPathSegments.pop();
        const { width, height } = treemapCanvasEl.getBoundingClientRect();
        const result = await invoke<LayoutResult>("zoom_in", { segments: currentPathSegments, width, height });
        currentRectangles = result.rectangles;

        const currentFullPath = [scanRootPath, ...currentPathSegments].join('/');

        await appWindow.setTitle(tr('window-title-viewing', {
            path: currentFullPath,
            size: formatBytes(result.total_size),
            items: result.total_items.toString()
        }));

        await renderTreemap();
    } catch (error) {
        console.error("Zoom out failed:", error);
        await showScanMessage("scan-failed-title", `${error}`);
    }
}


function resetZoom() {
    if (currentPathSegments.length > 0) {
        zoomIn([]);
    }
}

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

function hideTooltipAndDeselect() {
    if (treemapTooltipEl) treemapTooltipEl.style.display = 'none';
    if (selectedRectangle) {
        selectedRectangle = null;
        renderTreemap();
    }
}

async function handleCanvasClick(event: MouseEvent) {
    const clickedRect = getClickedRectangle(event);

    // Si on clique dans le vide, on cache l'infobulle et on désélectionne
    if (!clickedRect) {
        hideTooltipAndDeselect();
        return;
    }

    // Si on clique sur le même rectangle déjà sélectionné, on désélectionne (toggle)
    if (selectedRectangle && selectedRectangle.path === clickedRect.path) {
        hideTooltipAndDeselect();
        return;
    }

    selectedRectangle = clickedRect;
    await renderTreemap();

    if (treemapTooltipEl && treemapCanvasEl) {
        const sizeStr = formatBytes(selectedRectangle.size);
        const typeKey = selectedRectangle.is_directory ? 'tooltip-type-dir' : 'tooltip-type-file';

        // Calculer le pourcentage par rapport au total affiché (somme du niveau 0)
        const totalDisplayedSize = currentRectangles
            .filter(r => r && r.depth === 0)
            .reduce((sum, r) => sum + r.size, 0);
            
        const percentage = totalDisplayedSize > 0 ? (selectedRectangle.size / totalDisplayedSize) * 100 : 0;

        treemapTooltipEl.innerHTML = `
            <strong>${selectedRectangle.name === 'other-files-name' ? tr('other-files-name') : selectedRectangle.name}</strong>
            <span>${tr('tooltip-path-label')}:</span> ${selectedRectangle.path}<br>
            <span>${tr('tooltip-size-label')}:</span> ${sizeStr} (${percentage.toFixed(4)}%)<br>
            <span>${tr('tooltip-type-label')}:</span> ${tr(typeKey)}<br>
            <span>Pixels:</span> ${selectedRectangle.width.toFixed(1)}px x ${selectedRectangle.height.toFixed(1)}px
        `;
        const rect = treemapCanvasEl.getBoundingClientRect();
        treemapTooltipEl.style.left = `${event.clientX - rect.left + 10}px`;
        treemapTooltipEl.style.top = `${event.clientY - rect.top + 10}px`;
        treemapTooltipEl.style.display = 'block';
    }
}

function handleCanvasDblClick(event: MouseEvent) {
    const clickedRect = getClickedRectangle(event);
    if (!clickedRect || !clickedRect.is_directory || !scanRootPath) return;
    if (clickedRect.path === scanRootPath) return;

    if (clickedRect.path.startsWith(scanRootPath)) {
        let relativePath = clickedRect.path.substring(scanRootPath.length);
        if (relativePath.startsWith('/')) {
            relativePath = relativePath.substring(1);
        }
        const segments = relativePath.split('/').filter(s => s.length > 0 && s !== '.');
        if (segments.length > 0) {
            zoomIn(segments);
        }
    }
}

function getClickedRectangle(event: MouseEvent): Rectangle | null {
    if (!treemapCanvasEl) return null;
    const x = event.offsetX;
    const y = event.offsetY;
    let clickedRect: Rectangle | null = null;
    for (const r of currentRectangles) {
        if (!r) continue;
        if (x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height) {
            if (!clickedRect || (r.width * r.height < clickedRect.width * clickedRect.height)) {
                clickedRect = r;
            }
        }
    }
    return clickedRect;
}

// --- INITIALIZATION ---
window.addEventListener("DOMContentLoaded", async () => {
    // DOM elements
    scanPathInputEl = document.querySelector("#scan-path-input");
    treemapCanvasEl = document.querySelector("#treemap-canvas");
    treemapTooltipEl = document.querySelector("#treemap-tooltip");
    if (treemapCanvasEl) {
        treemapCtx = treemapCanvasEl.getContext('2d');
    }

    // Event listeners
    document.querySelector("#scan-form")?.addEventListener("submit", (e) => { e.preventDefault(); performScan(); });
    document.querySelector("#browse-btn")?.addEventListener("click", browse);
    document.querySelector("#zoom-out-btn")?.addEventListener("click", zoomOut);
    document.querySelector("#reset-zoom-btn")?.addEventListener("click", resetZoom);
    treemapCanvasEl?.addEventListener('click', handleCanvasClick);
    treemapCanvasEl?.addEventListener('dblclick', handleCanvasDblClick);
    window.addEventListener('resize', () => {
        if (scanRootPath && currentRectangles.length > 0) {
            zoomIn(currentPathSegments);
        }
    });

    // Backend event listeners
    listen('scan-progress', async (event) => {
        const payload = event.payload as { path: string, files: number, dirs: number };
        await showScanMessage("scanning-title", payload.path, payload.files, payload.dirs);
        await appWindow.setTitle(tr('window-title-scanning'));
    });
    listen('scan-complete', async () => {
        // Déclencher immédiatement la récupération du layout racine avec les dimensions actuelles
        await zoomIn([]);
    });
    listen('scan-error', async (event) => {
        console.error("Scan failed:", event.payload);
        await showScanMessage("scan-failed-title", `${event.payload}`);
    });

    // Load translations
    try {
        let locale = await invoke<string>("get_locale");
        const response = await fetch(`/locales/${locale}.json`);
        if (!response.ok) {
            locale = 'en';
            const fallbackResponse = await fetch(`/locales/en.json`);
            i18nMessages = await fallbackResponse.json();
        } else {
            i18nMessages = await response.json();
        }
    } catch (e) { console.error("Failed to load translations", e); }

    // Set initial texts
    document.querySelector('#path-label')!.textContent = tr('path-label');
    document.querySelector('#browse-btn')!.textContent = tr('browse-btn');
    document.querySelector('#scan-btn')!.textContent = tr('scan-btn');
    document.querySelector('#zoom-out-btn')!.textContent = tr('zoom-out-btn');
    document.querySelector('#reset-zoom-btn')!.textContent = tr('reset-btn');
    scanPathInputEl!.placeholder = tr('path-input-placeholder');
    await appWindow.setTitle(tr('window-title-default'));

    // Set initial path
    try {
        const defaultPath = await invoke<string>("get_default_scan_path");
        if (scanPathInputEl) {
            scanPathInputEl.value = defaultPath;
            scanRootPath = defaultPath;
        }
    } catch (error) {
        console.error("Failed to get default path:", error);
    }
});

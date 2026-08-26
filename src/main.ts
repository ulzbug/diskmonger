import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";

// --- INTERFACES ---
interface Rectangle {
    x: number;
    y: number;
    width: number;
    height: number;
    depth: number;
    path: string;
    name: string;
    is_directory: boolean;
    size: number;
}

// --- STATE MANAGEMENT ---
let scanPathInputEl: HTMLInputElement | null;
let treemapCanvasEl: HTMLCanvasElement | null;
let treemapCtx: CanvasRenderingContext2D | null = null;
let treemapTooltipEl: HTMLElement | null = null;

let currentRectangles: Rectangle[] = [];
let scanRootPath: string | null = null; // Mémorise la racine du scan initial
let currentPath: string | null = null; // Le chemin du dossier actuellement affiché
let selectedRectangle: Rectangle | null = null;

const appWindow = getCurrentWindow();

// --- HELPERS ---
function formatBytes(bytes: number, decimals = 2): string {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}


// --- UI RENDERING (CANVAS) ---

const COLORS = ['#FF7F7F', '#FFBF7F', '#FFFF00', '#7FFF7F', '#7FFFFF', '#BFBFFF', '#BFBFBF', '#FF7FFF'];
const BORDER_COLOR = '#000000';
const SELECTED_BORDER_COLOR = '#FFFFFF';

function renderTreemap() {
    if (!treemapCanvasEl || !treemapCtx) return;

    const { width, height } = treemapCanvasEl.getBoundingClientRect();
    treemapCanvasEl.width = width;
    treemapCanvasEl.height = height;
    
    treemapCtx.clearRect(0, 0, width, height);

    for (const rect of currentRectangles) {
        treemapCtx.fillStyle = COLORS[rect.depth % COLORS.length];
        treemapCtx.fillRect(rect.x, rect.y, rect.width, rect.height);
        
        if (selectedRectangle && rect.path === selectedRectangle.path) {
            treemapCtx.strokeStyle = SELECTED_BORDER_COLOR;
            treemapCtx.lineWidth = 2;
        } else {
            treemapCtx.strokeStyle = BORDER_COLOR;
            treemapCtx.lineWidth = 1;
        }
        treemapCtx.strokeRect(rect.x, rect.y, rect.width, rect.height);

        if (rect.is_directory) {
            treemapCtx.fillStyle = '#000000';
            treemapCtx.font = "bold 9px sans-serif";
            treemapCtx.fillText(rect.name, rect.x + 3, rect.y + 10);
        } else {
            if (rect.width > 35 && rect.height > 12) {
                treemapCtx.fillStyle = '#000000';
                treemapCtx.font = "8px sans-serif";
                treemapCtx.fillText(rect.name, rect.x + 4, rect.y + 10);
            }
        }
    }
}

function showScanMessage(text: string, subtext: string = "") {
    if (!treemapCtx || !treemapCanvasEl) return;
    const { width, height } = treemapCanvasEl.getBoundingClientRect();
    treemapCanvasEl.width = width;
    treemapCanvasEl.height = height;
    treemapCtx.clearRect(0, 0, width, height);
    treemapCtx.fillStyle = '#FFFFFF';
    treemapCtx.font = "18px sans-serif";
    treemapCtx.fillText(text, 50, 50);
    if (subtext) {
        treemapCtx.font = "14px sans-serif";
        treemapCtx.fillText(subtext, 50, 80);
    }
}


// --- EVENT HANDLERS & INITIALIZATION ---

async function performScan() {
    if (scanPathInputEl && treemapCanvasEl) {
        const path = scanPathInputEl.value;
        scanRootPath = path; // On mémorise la racine initiale
        currentPath = path;
        const { width, height } = treemapCanvasEl.getBoundingClientRect();
        showScanMessage("Scanning...", path);
        selectedRectangle = null;
        if (treemapTooltipEl) treemapTooltipEl.style.display = 'none';
        appWindow.setTitle(`DiskMonger - Scanning...`);
        invoke("scan", { path, width, height });
    }
}

async function zoomTo(path: string) {
    if (!treemapCanvasEl) return;
    try {
        const { width, height } = treemapCanvasEl.getBoundingClientRect();
        currentRectangles = await invoke("zoom_in", { path, width, height });
        selectedRectangle = null;
        if (treemapTooltipEl) treemapTooltipEl.style.display = 'none';
        
        const totalSize = currentRectangles.reduce((sum, r) => sum + r.size, 0);
        appWindow.setTitle(`DiskMonger - ${path} (${formatBytes(totalSize)}, ${currentRectangles.length} items)`);
        
        renderTreemap();
    } catch (error) {
        console.error("Zoom failed:", error);
        showScanMessage("Zoom failed!", `${error}`);
    }
}

// Remonter d'un niveau logique dans l'arborescence
function zoomOut() {
    if (currentPath && scanRootPath && currentPath !== scanRootPath) {
        const segments = currentPath.split('/');
        segments.pop(); // Enlever le dernier segment (le dossier actuel)
        const parentPath = segments.join('/') || '/'; // Reconstruire le chemin parent
        
        currentPath = parentPath;
        zoomTo(parentPath);
    }
}

// Revenir directement à la racine du scan initial
function resetZoom() {
    if (scanRootPath && currentPath !== scanRootPath) {
        currentPath = scanRootPath;
        zoomTo(scanRootPath);
    }
}

async function browse() {
    const result = await open({
        directory: true,
        multiple: false,
        defaultPath: currentPath || undefined,
    });

    if (typeof result === 'string' && scanPathInputEl) {
        scanPathInputEl.value = result;
        performScan();
    }
}

function getClickedRectangle(event: MouseEvent): Rectangle | null {
    if (!treemapCanvasEl) return null;
    const x = event.offsetX;
    const y = event.offsetY;

    let clickedRect: Rectangle | null = null;
    for (const r of currentRectangles) {
        if (x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height) {
            if (!clickedRect || (r.width * r.height < clickedRect.width * clickedRect.height)) {
                clickedRect = r;
            }
        }
    }
    return clickedRect;
}

function handleCanvasClick(event: MouseEvent) {
    selectedRectangle = getClickedRectangle(event);
    renderTreemap();

    if (selectedRectangle && treemapTooltipEl && treemapCanvasEl) {
        const sizeStr = formatBytes(selectedRectangle.size);
        const typeStr = selectedRectangle.is_directory ? 'Directory' : 'File';
        
        treemapTooltipEl.innerHTML = `
            <strong>${selectedRectangle.name}</strong>
            <span>Path:</span> ${selectedRectangle.path}<br>
            <span>Size:</span> ${sizeStr}<br>
            <span>Type:</span> ${typeStr}
        `;

        const rect = treemapCanvasEl.getBoundingClientRect();
        treemapTooltipEl.style.left = `${event.clientX - rect.left + 10}px`;
        treemapTooltipEl.style.top = `${event.clientY - rect.top + 10}px`;
        treemapTooltipEl.style.display = 'block';
    } else if (treemapTooltipEl) {
        treemapTooltipEl.style.display = 'none';
    }
}

function handleCanvasDblClick(event: MouseEvent) {
    const clickedRect = getClickedRectangle(event);
    if (clickedRect && clickedRect.is_directory) {
        currentPath = clickedRect.path; // On met à jour le chemin actuel
        zoomTo(currentPath);
    }
}

async function setupEventListeners() {
    await listen('scan-progress', (event) => {
        const payload = event.payload as { path: string };
        showScanMessage("Scanning...", payload.path);
        appWindow.setTitle(`DiskMonger - Scanning... ${payload.path}`);
    });

    await listen('scan-complete', (event) => {
        currentRectangles = event.payload as Rectangle[];
        console.log("Scan complete, received rectangles:", currentRectangles.length);
        
        const totalSize = currentRectangles.reduce((sum, r) => sum + r.size, 0);
        appWindow.setTitle(`DiskMonger - ${currentPath} (${formatBytes(totalSize)}, ${currentRectangles.length} items)`);
        
        renderTreemap();
    });

    await listen('scan-error', (event) => {
        console.error("Scan failed:", event.payload);
        showScanMessage("Scan failed!", `${event.payload}`);
    });
}

window.addEventListener("DOMContentLoaded", () => {
    scanPathInputEl = document.querySelector("#scan-path-input");
    treemapCanvasEl = document.querySelector("#treemap-canvas");
    treemapTooltipEl = document.querySelector("#treemap-tooltip");
    
    if (treemapCanvasEl) {
        treemapCtx = treemapCanvasEl.getContext('2d');
        treemapCanvasEl.addEventListener('click', handleCanvasClick);
        treemapCanvasEl.addEventListener('dblclick', handleCanvasDblClick);
    }

    document.querySelector("#scan-form")?.addEventListener("submit", (e) => { e.preventDefault(); performScan(); });
    document.querySelector("#browse-btn")?.addEventListener("click", browse);
    document.querySelector("#zoom-out-btn")?.addEventListener("click", zoomOut);
    document.querySelector("#reset-zoom-btn")?.addEventListener("click", resetZoom);
    
    window.addEventListener('resize', () => {
        if (currentPath) {
            zoomTo(currentPath);
        }
    });

    setupEventListeners();
});

import { useMemo } from "react";
import type { Viewport } from "../lib/types";
import styles from "./GridOverlay.module.css";

const COLS = 8;
const ROWS = 8;
const COL_LABELS = "ABCDEFGH";

interface GridOverlayProps {
    width: number;
    height: number;
    viewport: Viewport;
}

export function GridOverlay({ width, height, viewport }: GridOverlayProps) {
    const cellW = width / COLS;
    const cellH = height / ROWS;

    const colLabels = useMemo(
        () =>
            Array.from({ length: COLS }, (_, i) => ({
                label: COL_LABELS[i],
                x: cellW * i + cellW / 2,
            })),
        [cellW],
    );

    const rowLabels = useMemo(
        () =>
            Array.from({ length: ROWS }, (_, i) => ({
                label: String(i + 1),
                y: cellH * i + cellH / 2,
            })),
        [cellH],
    );

    // Column labels are positioned relative to the canvas area,
    // but the top/bottom strips extend 52px into each rail.
    // Offset labels by 52px so they align with the canvas grid.
    const railWidth = 52;

    return (
        <>
            {/* Column labels: top and bottom */}
            <div className={styles.top}>
                {colLabels.map((col) => (
                    <span key={col.label} className={styles.label} style={{ left: col.x + railWidth }}>
                        {col.label}
                    </span>
                ))}
            </div>
            <div className={styles.bottom}>
                {colLabels.map((col) => (
                    <span key={col.label} className={styles.label} style={{ left: col.x + railWidth }}>
                        {col.label}
                    </span>
                ))}
            </div>

            {/* Row labels: left and right */}
            <div className={styles.left}>
                {rowLabels.map((row) => (
                    <span key={row.label} className={styles.label} style={{ top: row.y }}>
                        {row.label}
                    </span>
                ))}
            </div>
            <div className={styles.right}>
                {rowLabels.map((row) => (
                    <span key={row.label} className={styles.label} style={{ top: row.y }}>
                        {row.label}
                    </span>
                ))}
            </div>
        </>
    );
}

/**
 * Convert a Battleship-style cell reference (e.g. "A4", "D1") to canvas coordinates.
 * Returns the center of the cell in canvas space.
 */
export function cellToCanvas(
    cell: string,
    viewportWidth: number,
    viewportHeight: number,
    viewport: Viewport,
): { x: number; y: number } {
    const col = cell.charCodeAt(0) - 65; // A=0, B=1, ...
    const row = parseInt(cell.slice(1), 10) - 1; // 1-based → 0-based

    const cellW = viewportWidth / COLS;
    const cellH = viewportHeight / ROWS;

    // Screen position of cell center
    const screenX = cellW * col + cellW / 2;
    const screenY = cellH * row + cellH / 2;

    // Convert screen → canvas
    const canvasX = (screenX - viewport.x) / viewport.scale;
    const canvasY = (screenY - viewport.y) / viewport.scale;

    return { x: canvasX, y: canvasY };
}

/**
 * Convert canvas coordinates to a Battleship-style cell reference.
 */
export function canvasToCell(
    canvasX: number,
    canvasY: number,
    viewportWidth: number,
    viewportHeight: number,
    viewport: Viewport,
): string {
    // Canvas → screen
    const screenX = canvasX * viewport.scale + viewport.x;
    const screenY = canvasY * viewport.scale + viewport.y;

    const cellW = viewportWidth / COLS;
    const cellH = viewportHeight / ROWS;

    const col = Math.floor(screenX / cellW);
    const row = Math.floor(screenY / cellH);

    const clampedCol = Math.max(0, Math.min(COLS - 1, col));
    const clampedRow = Math.max(0, Math.min(ROWS - 1, row));

    return `${COL_LABELS[clampedCol]}${clampedRow + 1}`;
}

/**
 * Build a viewport grid description for the AI system prompt.
 * Maps each cell to its canvas coordinate range.
 */
export function buildGridContext(
    viewportWidth: number,
    viewportHeight: number,
    viewport: Viewport,
): string {
    const cellW = viewportWidth / COLS;
    const cellH = viewportHeight / ROWS;

    const lines: string[] = [
        "The user's viewport is divided into an 8x8 grid (columns A-H, rows 1-8).",
        "Users may refer to positions using Battleship-style coordinates like 'A1' (top-left) or 'H8' (bottom-right).",
        "Grid cell canvas coordinate centers:",
    ];

    for (let r = 0; r < ROWS; r++) {
        const cells: string[] = [];
        for (let c = 0; c < COLS; c++) {
            const screenX = cellW * c + cellW / 2;
            const screenY = cellH * r + cellH / 2;
            const cx = Math.round((screenX - viewport.x) / viewport.scale);
            const cy = Math.round((screenY - viewport.y) / viewport.scale);
            cells.push(`${COL_LABELS[c]}${r + 1}=(${cx},${cy})`);
        }
        lines.push(cells.join(" "));
    }

    return lines.join("\n");
}

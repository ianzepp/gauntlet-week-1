import type { ReactNode } from "react";
import type { ToolType } from "../lib/types";
import { useBoardStore } from "../store/board";
import styles from "./ToolRail.module.css";

const TOOLS: { type: ToolType; label: string; icon: ReactNode }[] = [
    {
        type: "select",
        label: "Select",
        icon: (
            <svg viewBox="0 0 20 20">
                <path d="M4 2 L4 16 L8 12 L12 18 L14 17 L10 11 L15 11 Z" />
            </svg>
        ),
    },
    {
        type: "sticky",
        label: "Note",
        icon: (
            <svg viewBox="0 0 20 20">
                <rect x="2" y="2" width="16" height="16" />
                <line x1="5" y1="7" x2="15" y2="7" />
                <line x1="5" y1="11" x2="12" y2="11" />
            </svg>
        ),
    },
    {
        type: "rectangle",
        label: "Rect",
        icon: (
            <svg viewBox="0 0 20 20">
                <rect x="2" y="4" width="16" height="12" />
            </svg>
        ),
    },
    {
        type: "ellipse",
        label: "Ellipse",
        icon: (
            <svg viewBox="0 0 20 20">
                <ellipse cx="10" cy="10" rx="8" ry="6" />
            </svg>
        ),
    },
];

export function ToolRail() {
    const activeTool = useBoardStore((s) => s.activeTool);
    const setTool = useBoardStore((s) => s.setTool);

    return (
        <div className={styles.rail}>
            {TOOLS.map((tool) => (
                <button
                    key={tool.type}
                    type="button"
                    className={`${styles.toolButton} ${activeTool === tool.type ? styles.active : ""}`}
                    onClick={() => setTool(tool.type)}
                    title={tool.label}
                >
                    {tool.icon}
                </button>
            ))}
            <div className={styles.separator} />
        </div>
    );
}

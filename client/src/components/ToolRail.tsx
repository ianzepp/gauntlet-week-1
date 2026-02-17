import type { ReactNode } from "react";
import type { ToolType } from "../lib/types";
import { useBoardStore } from "../store/board";
import styles from "./ToolRail.module.css";

interface ToolDef {
    type: ToolType;
    label: string;
    icon: ReactNode;
    disabled?: boolean;
}

const TOOLS: ToolDef[] = [
    {
        type: "select",
        label: "Select",
        icon: (
            <svg viewBox="0 0 20 20">
                <path d="M4 2 L4 16 L8 12 L12 18 L14 17 L10 11 L15 11 Z" />
            </svg>
        ),
    },
];

const SHAPE_TOOLS: ToolDef[] = [
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
        disabled: true,
        icon: (
            <svg viewBox="0 0 20 20">
                <ellipse cx="10" cy="10" rx="8" ry="6" />
            </svg>
        ),
    },
    {
        type: "line",
        label: "Line",
        disabled: true,
        icon: (
            <svg viewBox="0 0 20 20">
                <line x1="3" y1="17" x2="17" y2="3" />
            </svg>
        ),
    },
    {
        type: "connector",
        label: "Arrow",
        disabled: true,
        icon: (
            <svg viewBox="0 0 20 20">
                <line x1="3" y1="17" x2="17" y2="3" />
                <polyline points="10,3 17,3 17,10" />
            </svg>
        ),
    },
    {
        type: "text",
        label: "Text",
        disabled: true,
        icon: (
            <svg viewBox="0 0 20 20">
                <line x1="4" y1="4" x2="16" y2="4" />
                <line x1="10" y1="4" x2="10" y2="17" />
                <line x1="7" y1="17" x2="13" y2="17" />
            </svg>
        ),
    },
];

const DRAW_TOOLS: ToolDef[] = [
    {
        type: "draw",
        label: "Draw",
        disabled: true,
        icon: (
            <svg viewBox="0 0 20 20">
                <path d="M3 17 L14 6 L16 4 L17 3 L14 6" />
                <path d="M14 6 L16 8" />
                <line x1="3" y1="17" x2="5" y2="15" />
            </svg>
        ),
    },
    {
        type: "eraser",
        label: "Eraser",
        disabled: true,
        icon: (
            <svg viewBox="0 0 20 20">
                <path d="M8 16 L3 11 L11 3 L18 10 L13 16 Z" />
                <line x1="3" y1="16" x2="13" y2="16" />
            </svg>
        ),
    },
];

function ToolGroup({ tools }: { tools: ToolDef[] }) {
    const activeTool = useBoardStore((s) => s.activeTool);
    const setTool = useBoardStore((s) => s.setTool);

    return (
        <>
            {tools.map((tool) => (
                <button
                    key={tool.type}
                    type="button"
                    className={`${styles.toolButton} ${activeTool === tool.type ? styles.active : ""} ${tool.disabled ? styles.disabled : ""}`}
                    onClick={() => !tool.disabled && setTool(tool.type)}
                    title={tool.disabled ? `${tool.label} (coming soon)` : tool.label}
                    disabled={tool.disabled}
                >
                    {tool.icon}
                </button>
            ))}
        </>
    );
}

export function ToolRail() {
    return (
        <div className={styles.rail}>
            <ToolGroup tools={TOOLS} />
            <div className={styles.separator} />
            <ToolGroup tools={SHAPE_TOOLS} />
            <div className={styles.separator} />
            <ToolGroup tools={DRAW_TOOLS} />
        </div>
    );
}

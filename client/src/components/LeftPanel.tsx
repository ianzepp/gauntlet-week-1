import { useCallback, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { ToolType } from "../lib/types";
import { useBoardStore } from "../store/board";
import { InspectorPanel } from "./InspectorPanel";
import styles from "./LeftPanel.module.css";
import { ToolStrip } from "./ToolStrip";

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

/** Tools that open a strip flyout instead of setting activeTool */
const STRIP_TOOLS = new Set<ToolType>(["rectangle"]);

function ToolGroup({
    tools,
    openStrip,
    onStripToggle,
}: {
    tools: ToolDef[];
    openStrip: ToolType | null;
    onStripToggle: (type: ToolType, el: HTMLButtonElement) => void;
}) {
    const activeTool = useBoardStore((s) => s.activeTool);
    const setTool = useBoardStore((s) => s.setTool);

    return (
        <>
            {tools.map((tool) => {
                const isStripTool = STRIP_TOOLS.has(tool.type);
                const isActive = isStripTool
                    ? openStrip === tool.type
                    : activeTool === tool.type;

                return (
                    <button
                        key={tool.type}
                        type="button"
                        className={`${styles.toolButton} ${isActive ? styles.toolButtonActive : ""} ${tool.disabled ? styles.toolButtonDisabled : ""}`}
                        onClick={(e) => {
                            if (tool.disabled) return;
                            if (isStripTool) {
                                onStripToggle(tool.type, e.currentTarget);
                            } else {
                                setTool(tool.type);
                            }
                        }}
                        title={tool.disabled ? `${tool.label} (coming soon)` : tool.label}
                        disabled={tool.disabled}
                    >
                        {tool.icon}
                    </button>
                );
            })}
        </>
    );
}

export function LeftPanel() {
    const expanded = useBoardStore((s) => s.leftPanelExpanded);
    const collapseLeftPanel = useBoardStore((s) => s.collapseLeftPanel);
    const expandLeftPanel = useBoardStore((s) => s.expandLeftPanel);
    const [openStrip, setOpenStrip] = useState<ToolType | null>(null);
    const railRef = useRef<HTMLDivElement | null>(null);
    const stripButtonRef = useRef<HTMLButtonElement | null>(null);
    const [stripLeft, setStripLeft] = useState(52);
    const [stripTop, setStripTop] = useState(0);

    const updateStripPosition = useCallback(() => {
        const buttonEl = stripButtonRef.current;
        if (!buttonEl) return;
        const buttonRect = buttonEl.getBoundingClientRect();
        const railRect = railRef.current?.getBoundingClientRect();
        setStripLeft(railRect?.right ?? buttonRect.right);
        setStripTop(buttonRect.top);
    }, []);

    const handleStripToggle = (type: ToolType, el: HTMLButtonElement) => {
        if (openStrip === type) {
            setOpenStrip(null);
            stripButtonRef.current = null;
        } else {
            stripButtonRef.current = el;
            setOpenStrip(type);
            updateStripPosition();
        }
    };

    useEffect(() => {
        if (!openStrip) return;
        updateStripPosition();

        const onLayoutChange = () => updateStripPosition();
        window.addEventListener("resize", onLayoutChange);
        window.addEventListener("scroll", onLayoutChange, true);
        return () => {
            window.removeEventListener("resize", onLayoutChange);
            window.removeEventListener("scroll", onLayoutChange, true);
        };
    }, [expanded, openStrip, updateStripPosition]);

    return (
        <div className={styles.wrapper}>
            {expanded && (
                <div className={styles.panel}>
                    <div className={styles.header}>
                        <span className={styles.title}>Inspector</span>
                        <button
                            type="button"
                            className={styles.closeButton}
                            onClick={collapseLeftPanel}
                        >
                            ✕
                        </button>
                    </div>
                    <div className={styles.content}>
                        <InspectorPanel />
                    </div>
                </div>
            )}
            <div ref={railRef} className={styles.rail}>
                <ToolGroup tools={TOOLS} openStrip={openStrip} onStripToggle={handleStripToggle} />
                <div className={styles.separator} />
                <ToolGroup tools={SHAPE_TOOLS} openStrip={openStrip} onStripToggle={handleStripToggle} />
                <div className={styles.separator} />
                <ToolGroup tools={DRAW_TOOLS} openStrip={openStrip} onStripToggle={handleStripToggle} />
                <div className={styles.railSpacer} />
                <button
                    type="button"
                    className={styles.railToggle}
                    onClick={() =>
                        expanded ? collapseLeftPanel() : expandLeftPanel("inspector")
                    }
                    title={expanded ? "Collapse panel" : "Expand panel"}
                >
                    <svg viewBox="0 0 20 20">
                        {expanded ? (
                            <path d="M13 4 L7 10 L13 16" />
                        ) : (
                            <path d="M7 4 L13 10 L7 16" />
                        )}
                    </svg>
                </button>
            </div>
            {openStrip && (
                <div
                    className={styles.stripAnchor}
                    style={{ top: stripTop, left: stripLeft }}
                >
                    <ToolStrip onClose={() => {
                        setOpenStrip(null);
                        stripButtonRef.current = null;
                    }}
                    />
                </div>
            )}
        </div>
    );
}

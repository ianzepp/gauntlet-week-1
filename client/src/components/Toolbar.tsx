import type { ToolType } from "../lib/types";
import { useBoardStore } from "../store/board";
import styles from "./Toolbar.module.css";

const TOOLS: { type: ToolType; label: string; icon: string }[] = [
    { type: "select", label: "Select", icon: "\u25B3" },
    { type: "sticky", label: "Sticky", icon: "\u25A1" },
    { type: "rectangle", label: "Rect", icon: "\u25AD" },
    { type: "ellipse", label: "Ellipse", icon: "\u25CB" },
];

export function Toolbar() {
    const activeTool = useBoardStore((s) => s.activeTool);
    const setTool = useBoardStore((s) => s.setTool);
    const aiPanelOpen = useBoardStore((s) => s.aiPanelOpen);
    const toggleAiPanel = useBoardStore((s) => s.toggleAiPanel);

    const toggleDarkMode = () => {
        const html = document.documentElement;
        const isDark = html.classList.toggle("dark-mode");
        localStorage.setItem("collaboard_dark", isDark ? "true" : "false");
    };

    return (
        <div className={styles.toolbar}>
            <div className={styles.tools}>
                {TOOLS.map((tool) => (
                    <button
                        key={tool.type}
                        type="button"
                        className={`${styles.toolButton} ${activeTool === tool.type ? styles.active : ""}`}
                        onClick={() => setTool(tool.type)}
                    >
                        <span className={styles.toolIcon}>{tool.icon}</span>
                        <span className={styles.toolLabel}>{tool.label}</span>
                    </button>
                ))}
            </div>
            <div className={styles.separator} />
            <div className={styles.right}>
                <button
                    type="button"
                    className={`${styles.toolButton} ${aiPanelOpen ? styles.active : ""}`}
                    onClick={toggleAiPanel}
                >
                    <span className={styles.toolIcon}>{"\u2726"}</span>
                    <span className={styles.toolLabel}>AI</span>
                </button>
                <button
                    type="button"
                    className={styles.toolButton}
                    onClick={toggleDarkMode}
                >
                    <span className={styles.toolIcon}>{"\u263D"}</span>
                    <span className={styles.toolLabel}>Theme</span>
                </button>
            </div>
        </div>
    );
}

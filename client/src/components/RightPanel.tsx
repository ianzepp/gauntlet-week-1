import { useEffect, useRef } from "react";
import { useBoardStore } from "../store/board";
import type { RightTab } from "../store/board";
import { AiPanel } from "./AiPanel";
import { InspectorPanel } from "./InspectorPanel";
import styles from "./RightPanel.module.css";

const TABS: { id: RightTab; label: string }[] = [
    { id: "inspector", label: "Inspector" },
    { id: "ai", label: "Field Notes" },
];

export function RightPanel() {
    const activeTab = useBoardStore((s) => s.activeRightTab);
    const setRightTab = useBoardStore((s) => s.setRightTab);
    const closeRightPanel = useBoardStore((s) => s.closeRightPanel);
    const selection = useBoardStore((s) => s.selection);
    const prevSelectionSize = useRef(selection.size);

    // Auto-raise inspector when selection changes
    useEffect(() => {
        if (selection.size > 0 && prevSelectionSize.current === 0) {
            setRightTab("inspector");
        }
        prevSelectionSize.current = selection.size;
    }, [selection.size, setRightTab]);

    return (
        <div className={styles.panel}>
            <div className={styles.header}>
                {TABS.map((tab) => (
                    <button
                        key={tab.id}
                        type="button"
                        className={`${styles.tab} ${activeTab === tab.id ? styles.activeTab : ""}`}
                        onClick={() => setRightTab(tab.id)}
                    >
                        {tab.label}
                    </button>
                ))}
                <button
                    type="button"
                    className={styles.closeButton}
                    onClick={closeRightPanel}
                >
                    ✕
                </button>
            </div>
            <div className={styles.content}>
                {activeTab === "inspector" && <InspectorPanel />}
                {activeTab === "ai" && <AiPanel />}
            </div>
        </div>
    );
}

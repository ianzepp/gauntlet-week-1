import React, { useEffect, useRef } from "react";
import { useBoardStore } from "../store/board";
import type { RightTab } from "../store/board";
import { AiPanel } from "./AiPanel";
import { InspectorPanel } from "./InspectorPanel";
import styles from "./RightPanel.module.css";

const TABS: { id: RightTab; label: string; icon: React.ReactNode }[] = [
    {
        id: "inspector",
        label: "Inspector",
        icon: (
            <svg viewBox="0 0 20 20">
                <rect x="3" y="3" width="14" height="14" rx="2" />
                <line x1="3" y1="10" x2="17" y2="10" />
                <line x1="10" y1="10" x2="10" y2="17" />
            </svg>
        ),
    },
    {
        id: "ai",
        label: "Field Notes",
        icon: (
            <svg viewBox="0 0 20 20">
                <path d="M10 2 L12 8 L18 8 L13 12 L15 18 L10 14 L5 18 L7 12 L2 8 L8 8 Z" />
            </svg>
        ),
    },
];

export function RightPanel() {
    const activeTab = useBoardStore((s) => s.activeRightTab);
    const expanded = useBoardStore((s) => s.rightPanelExpanded);
    const setRightTab = useBoardStore((s) => s.setRightTab);
    const collapseRightPanel = useBoardStore((s) => s.collapseRightPanel);
    const expandRightPanel = useBoardStore((s) => s.expandRightPanel);
    const selection = useBoardStore((s) => s.selection);
    const prevSelectionSize = useRef(selection.size);

    // Auto-raise inspector when selection changes
    useEffect(() => {
        if (selection.size > 0 && prevSelectionSize.current === 0) {
            setRightTab("inspector");
        }
        prevSelectionSize.current = selection.size;
    }, [selection.size, setRightTab]);

    const handleRailClick = (tabId: RightTab) => {
        if (expanded && activeTab === tabId) {
            collapseRightPanel();
        } else {
            expandRightPanel(tabId);
        }
    };

    return (
        <div className={styles.wrapper}>
            <div className={styles.rail}>
                {TABS.map((tab) => (
                    <button
                        key={tab.id}
                        type="button"
                        className={`${styles.railButton} ${expanded && activeTab === tab.id ? styles.railButtonActive : ""}`}
                        onClick={() => handleRailClick(tab.id)}
                        title={tab.label}
                    >
                        {tab.icon}
                    </button>
                ))}
            </div>
            {expanded && (
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
                            onClick={collapseRightPanel}
                        >
                            ✕
                        </button>
                    </div>
                    <div className={styles.content}>
                        {activeTab === "inspector" && <InspectorPanel />}
                        {activeTab === "ai" && <AiPanel />}
                    </div>
                </div>
            )}
        </div>
    );
}

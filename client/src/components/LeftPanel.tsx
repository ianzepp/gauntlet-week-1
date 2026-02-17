import React, { useEffect, useRef } from "react";
import { useBoardStore } from "../store/board";
import type { LeftTab } from "../store/board";
import { InspectorPanel } from "./InspectorPanel";
import styles from "./LeftPanel.module.css";

const TABS: { id: LeftTab; label: string; icon: React.ReactNode }[] = [
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
];

export function LeftPanel() {
    const activeTab = useBoardStore((s) => s.activeLeftTab);
    const expanded = useBoardStore((s) => s.leftPanelExpanded);
    const collapseLeftPanel = useBoardStore((s) => s.collapseLeftPanel);
    const expandLeftPanel = useBoardStore((s) => s.expandLeftPanel);
    const selection = useBoardStore((s) => s.selection);
    const prevSelectionSize = useRef(selection.size);

    // Auto-raise inspector when selection changes
    useEffect(() => {
        if (selection.size > 0 && prevSelectionSize.current === 0) {
            expandLeftPanel("inspector");
        }
        prevSelectionSize.current = selection.size;
    }, [selection.size, expandLeftPanel]);

    const handleRailClick = (tabId: LeftTab) => {
        if (expanded && activeTab === tabId) {
            collapseLeftPanel();
        } else {
            expandLeftPanel(tabId);
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
                <div className={styles.railSpacer} />
                <button
                    type="button"
                    className={styles.railToggle}
                    onClick={() =>
                        expanded ? collapseLeftPanel() : expandLeftPanel(activeTab)
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
            {expanded && (
                <div className={styles.panel}>
                    <div className={styles.header}>
                        <span className={styles.title}>
                            {TABS.find((t) => t.id === activeTab)?.label}
                        </span>
                        <button
                            type="button"
                            className={styles.closeButton}
                            onClick={collapseLeftPanel}
                        >
                            ✕
                        </button>
                    </div>
                    <div className={styles.content}>
                        {activeTab === "inspector" && <InspectorPanel />}
                    </div>
                </div>
            )}
        </div>
    );
}

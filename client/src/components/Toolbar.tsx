import { useCallback, useRef, useState } from "react";
import { useBoardStore } from "../store/board";
import styles from "./Toolbar.module.css";
import { UserFieldReport } from "./UserFieldReport";

interface ToolbarProps {
    onBack?: () => void;
    onLogout?: () => void;
}

export function Toolbar({ onBack, onLogout }: ToolbarProps) {
    const presence = useBoardStore((s) => s.presence);
    const user = useBoardStore((s) => s.user);

    const [activeReport, setActiveReport] = useState<{
        userId: string;
        anchorX: number;
    } | null>(null);

    const chipRefs = useRef<Map<string, HTMLSpanElement>>(new Map());

    const allUsers = [
        ...(user ? [{ id: user.id, name: user.name, color: user.color }] : []),
        ...Array.from(presence.values()).map((p) => ({
            id: p.user_id,
            name: p.name,
            color: p.color,
        })),
    ];

    const handleChipClick = useCallback((userId: string) => {
        const el = chipRefs.current.get(userId);
        if (!el) return;
        const rect = el.getBoundingClientRect();
        setActiveReport({
            userId,
            anchorX: rect.left + rect.width / 2,
        });
    }, []);

    return (
        <div className={styles.toolbar}>
            <div className={styles.left}>
                {onBack && (
                    <button
                        type="button"
                        className={styles.actionButton}
                        onClick={onBack}
                        title="Back to Dashboard"
                    >
                        <svg viewBox="0 0 20 20" className={styles.actionIcon}>
                            <path d="M12 4 L6 10 L12 16" />
                        </svg>
                    </button>
                )}
                <span className={styles.boardName}>CollabBoard</span>
                <span className={styles.userDivider} />
                {allUsers.map((u) => (
                    <span
                        key={u.id}
                        ref={(el) => {
                            if (el) chipRefs.current.set(u.id, el);
                            else chipRefs.current.delete(u.id);
                        }}
                        className={styles.presenceChip}
                        style={{ borderColor: u.color }}
                        title={u.name}
                        onClick={() => handleChipClick(u.id)}
                    >
                        <span
                            className={styles.presenceDot}
                            style={{ background: u.color }}
                        />
                        {u.name}
                    </span>
                ))}
            </div>
            <div className={styles.right}>
                <button
                    type="button"
                    className={styles.actionButton}
                    onClick={onLogout}
                    title="Log Out"
                >
                    <svg viewBox="0 0 20 20" className={styles.actionIcon}>
                        <path d="M9 4 L4 4 L4 16 L9 16" />
                        <path d="M10 10 L17 10" />
                        <path d="M14 7 L17 10 L14 13" />
                    </svg>
                </button>
            </div>

            {activeReport && (
                <UserFieldReport
                    userId={activeReport.userId}
                    anchorX={activeReport.anchorX}
                    direction="down"
                    onClose={() => setActiveReport(null)}
                />
            )}
        </div>
    );
}

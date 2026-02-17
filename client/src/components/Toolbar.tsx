import { useBoardStore } from "../store/board";
import styles from "./Toolbar.module.css";

interface ToolbarProps {
    onBack?: () => void;
}

export function Toolbar({ onBack }: ToolbarProps) {
    const presence = useBoardStore((s) => s.presence);
    const user = useBoardStore((s) => s.user);

    const toggleDarkMode = () => {
        const html = document.documentElement;
        const isDark = html.classList.toggle("dark-mode");
        localStorage.setItem("collaboard_dark", isDark ? "true" : "false");
    };

    const allUsers = [
        ...(user ? [{ id: user.id, name: user.name, color: user.color }] : []),
        ...Array.from(presence.values()).map((p) => ({
            id: p.user_id,
            name: p.name,
            color: p.color,
        })),
    ];

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
            </div>
            <div className={styles.center}>
                {allUsers.map((u) => (
                    <span
                        key={u.id}
                        className={styles.presenceChip}
                        style={{ borderColor: u.color }}
                        title={u.name}
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
                    onClick={toggleDarkMode}
                    title="Toggle Theme"
                >
                    <svg viewBox="0 0 20 20" className={styles.actionIcon}>
                        <path d="M10 3 A7 7 0 1 0 10 17 A5 5 0 1 1 10 3" />
                    </svg>
                </button>
            </div>
        </div>
    );
}

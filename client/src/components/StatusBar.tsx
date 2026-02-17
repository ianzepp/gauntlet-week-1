import { useBoardStore } from "../store/board";
import styles from "./StatusBar.module.css";

function IconMouse() {
    return (
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M3 3l7.07 16.97 2.51-7.39 7.39-2.51L3 3z" />
        </svg>
    );
}

function IconCrosshair() {
    return (
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="2" x2="12" y2="6" />
            <line x1="12" y1="18" x2="12" y2="22" />
            <line x1="2" y1="12" x2="6" y2="12" />
            <line x1="18" y1="12" x2="22" y2="12" />
        </svg>
    );
}

export function StatusBar() {
    const objects = useBoardStore((s) => s.objects);
    const viewport = useBoardStore((s) => s.viewport);
    const connectionStatus = useBoardStore((s) => s.connectionStatus);
    const user = useBoardStore((s) => s.user);
    const boardName = useBoardStore((s) => s.boardName);
    const cursorPosition = useBoardStore((s) => s.cursorPosition);
    const viewportCenter = useBoardStore((s) => s.viewportCenter);

    const objectCount = objects.size;
    const zoom = Math.round(viewport.scale * 100);
    const isConnected = connectionStatus === "connected";

    return (
        <div className={styles.statusBar}>
            <div className={styles.section}>
                <span className={styles.item}>
                    <span
                        className={`${styles.dot} ${isConnected ? styles.connected : ""}`}
                    />
                </span>
                {boardName && (
                    <>
                        <span className={styles.divider} />
                        <span className={styles.boardName}>{boardName}</span>
                    </>
                )}
                <span className={styles.divider} />
                <span className={styles.item}>
                    {objectCount} {objectCount === 1 ? "obj" : "objs"}
                </span>
            </div>
            <div className={styles.section}>
                {cursorPosition && (
                    <span className={styles.item}>
                        <IconMouse />
                        ({cursorPosition.x}, {cursorPosition.y})
                    </span>
                )}
                <span className={styles.divider} />
                <span className={styles.item}>
                    <IconCrosshair />
                    ({viewportCenter.x}, {viewportCenter.y})
                </span>
                <span className={styles.divider} />
                {user && (
                    <span className={styles.userChip}>
                        <span
                            className={styles.userDot}
                            style={{ background: user.color }}
                        />
                        {user.name}
                    </span>
                )}
                <span className={styles.divider} />
                <span className={styles.item}>{zoom}%</span>
            </div>
        </div>
    );
}

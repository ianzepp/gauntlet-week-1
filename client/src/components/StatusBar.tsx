import { useBoardStore } from "../store/board";
import styles from "./StatusBar.module.css";

export function StatusBar() {
    const objects = useBoardStore((s) => s.objects);
    const viewport = useBoardStore((s) => s.viewport);
    const connectionStatus = useBoardStore((s) => s.connectionStatus);
    const user = useBoardStore((s) => s.user);
    const boardName = useBoardStore((s) => s.boardName);

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

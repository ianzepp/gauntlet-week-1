import { useBoardStore } from "../store/board";
import styles from "./StatusBar.module.css";

export function StatusBar() {
    const objects = useBoardStore((s) => s.objects);
    const viewport = useBoardStore((s) => s.viewport);
    const connectionStatus = useBoardStore((s) => s.connectionStatus);
    const presence = useBoardStore((s) => s.presence);
    const user = useBoardStore((s) => s.user);
    const boardId = useBoardStore((s) => s.boardId);

    const objectCount = objects.size;
    const zoom = Math.round(viewport.scale * 100);
    const isConnected = connectionStatus === "connected";

    const allUsers = [
        ...(user ? [{ id: user.id, name: user.name, color: user.color }] : []),
        ...Array.from(presence.values()).map((p) => ({
            id: p.user_id,
            name: p.name,
            color: p.color,
        })),
    ];

    return (
        <div className={styles.statusBar}>
            <div className={styles.section}>
                <span className={styles.item}>
                    <span
                        className={`${styles.dot} ${isConnected ? styles.connected : ""}`}
                    />
                </span>
                {boardId && (
                    <>
                        <span className={styles.divider} />
                        <span className={styles.boardName}>{boardId}</span>
                    </>
                )}
                <span className={styles.divider} />
                <span className={styles.item}>
                    {objectCount} {objectCount === 1 ? "obj" : "objs"}
                </span>
            </div>
            <div className={styles.section}>
                {allUsers.map((u) => (
                    <span key={u.id} className={styles.userChip}>
                        <span
                            className={styles.userDot}
                            style={{ background: u.color }}
                        />
                        {u.name}
                    </span>
                ))}
                <span className={styles.divider} />
                <span className={styles.item}>{zoom}%</span>
            </div>
        </div>
    );
}

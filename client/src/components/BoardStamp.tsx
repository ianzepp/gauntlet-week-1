import { useBoardStore } from "../store/board";
import styles from "./BoardStamp.module.css";

export function BoardStamp() {
    const boardId = useBoardStore((s) => s.boardId);
    const objects = useBoardStore((s) => s.objects);
    const connectionStatus = useBoardStore((s) => s.connectionStatus);
    const presence = useBoardStore((s) => s.presence);
    const user = useBoardStore((s) => s.user);

    const isConnected = connectionStatus === "connected";
    const userCount = (user ? 1 : 0) + presence.size;

    return (
        <div className={styles.stamp}>
            <div className={styles.stampLabel}>Station Log</div>
            <div className={styles.stampTitle}>{boardId ?? "Untitled"}</div>
            <div className={styles.stampMeta}>
                <div className={styles.stampRow}>
                    <span className={styles.stampKey}>Objects</span>
                    <span className={styles.stampValue}>{objects.size}</span>
                </div>
                <div className={styles.stampRow}>
                    <span className={styles.stampKey}>Users</span>
                    <span className={styles.stampValue}>{userCount}</span>
                </div>
                <div className={styles.stampRow}>
                    <span className={styles.stampKey}>Status</span>
                    <span className={styles.stampValue}>
                        <span
                            className={`${styles.statusDot} ${isConnected ? styles.active : styles.offline}`}
                        />
                        {isConnected ? "Active" : "Offline"}
                    </span>
                </div>
            </div>
        </div>
    );
}

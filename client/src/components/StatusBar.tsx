import { useBoardStore } from "../store/board";
import styles from "./StatusBar.module.css";

const STATUS_LABELS: Record<string, string> = {
    connected: "Connected",
    connecting: "Connecting...",
    disconnected: "Offline",
};

export function StatusBar() {
    const objects = useBoardStore((s) => s.objects);
    const viewport = useBoardStore((s) => s.viewport);
    const connectionStatus = useBoardStore((s) => s.connectionStatus);
    const presence = useBoardStore((s) => s.presence);

    const objectCount = objects.size;
    const zoom = Math.round(viewport.scale * 100);
    const userCount = presence.size + 1; // +1 for self
    const isConnected = connectionStatus === "connected";

    return (
        <div className={styles.statusBar}>
            <div className={styles.section}>
                <span className={styles.item}>
                    <span className={`${styles.dot} ${isConnected ? styles.connected : ""}`} />
                    {STATUS_LABELS[connectionStatus] ?? "Offline"}
                </span>
                {isConnected && (
                    <>
                        <span className={styles.divider} />
                        <span className={styles.item}>
                            {userCount} {userCount === 1 ? "user" : "users"}
                        </span>
                    </>
                )}
                <span className={styles.divider} />
                <span className={styles.item}>
                    {objectCount} {objectCount === 1 ? "object" : "objects"}
                </span>
            </div>
            <div className={styles.section}>
                <span className={styles.item}>
                    {viewport.x.toFixed(0)}, {viewport.y.toFixed(0)}
                </span>
                <span className={styles.divider} />
                <span className={styles.item}>{zoom}%</span>
            </div>
        </div>
    );
}

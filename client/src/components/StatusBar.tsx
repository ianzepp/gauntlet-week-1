import { useBoardStore } from "../store/board";
import styles from "./StatusBar.module.css";

export function StatusBar() {
    const objects = useBoardStore((s) => s.objects);
    const viewport = useBoardStore((s) => s.viewport);

    const objectCount = objects.size;
    const zoom = Math.round(viewport.scale * 100);

    return (
        <div className={styles.statusBar}>
            <div className={styles.section}>
                <span className={styles.item}>
                    <span className={styles.dot} />
                    Offline
                </span>
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

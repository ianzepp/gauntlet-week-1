import { useBoardStore } from "../store/board";
import styles from "./InspectorPanel.module.css";

const NOTE_COLORS: { name: string; light: string; dark: string }[] = [
    { name: "Cream", light: "#F5F0E8", dark: "#2C2824" },
    { name: "Sage", light: "#B8C5B0", dark: "#3A4436" },
    { name: "Terracotta", light: "#C4A882", dark: "#4A3D30" },
    { name: "Slate", light: "#9AA3AD", dark: "#343A40" },
    { name: "Dust", light: "#C2A8A0", dark: "#443838" },
    { name: "Gold", light: "#C9B97A", dark: "#3E3A28" },
    { name: "Stone", light: "#A8A298", dark: "#343230" },
    { name: "Moss", light: "#8B9E7E", dark: "#2E3828" },
];

function useIsDark() {
    return document.documentElement.classList.contains("dark-mode");
}

export function InspectorPanel() {
    const selection = useBoardStore((s) => s.selection);
    const objects = useBoardStore((s) => s.objects);
    const updateObject = useBoardStore((s) => s.updateObject);
    const frameClient = useBoardStore((s) => s.frameClient);
    const isDark = useIsDark();

    const selectedIds = Array.from(selection);
    const selectedObjects = selectedIds
        .map((id) => objects.get(id))
        .filter(Boolean);

    if (selectedObjects.length === 0) {
        return (
            <div className={styles.panel}>
                <div className={styles.empty}>
                    <span className={styles.emptyLabel}>No selection</span>
                </div>
            </div>
        );
    }

    if (selectedObjects.length > 1) {
        return (
            <div className={styles.panel}>
                <div className={styles.section}>
                    <span className={styles.objectKind}>
                        {selectedObjects.length} objects
                    </span>
                </div>
            </div>
        );
    }

    const obj = selectedObjects[0]!;
    const kindLabel = obj.kind.replace("_", " ");
    const currentColor = (obj.props.color as string) || "#F5F0E8";

    const handleColorChange = (color: string) => {
        updateObject(obj.id, {
            props: { ...obj.props, color },
        });
        if (frameClient) {
            frameClient.send({
                id: crypto.randomUUID(),
                parent_id: null,
                ts: Date.now(),
                board_id: obj.board_id,
                from: "",
                syscall: "object:update",
                status: "request",
                data: {
                    id: obj.id,
                    props: { ...obj.props, color },
                },
            });
        }
    };

    return (
        <div className={styles.panel}>
            <div className={styles.section}>
                <span className={styles.objectKind}>{kindLabel}</span>
            </div>

            <div className={styles.section}>
                <span className={styles.sectionTitle}>Position</span>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>X</span>
                    <span className={styles.rowValue}>
                        {Math.round(obj.x)}
                    </span>
                </div>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>Y</span>
                    <span className={styles.rowValue}>
                        {Math.round(obj.y)}
                    </span>
                </div>
            </div>

            <div className={styles.section}>
                <span className={styles.sectionTitle}>Size</span>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>W</span>
                    <span className={styles.rowValue}>
                        {Math.round(obj.width)}
                    </span>
                </div>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>H</span>
                    <span className={styles.rowValue}>
                        {Math.round(obj.height)}
                    </span>
                </div>
                {obj.rotation !== 0 && (
                    <div className={styles.row}>
                        <span className={styles.rowLabel}>Rot</span>
                        <span className={styles.rowValue}>
                            {Math.round(obj.rotation)}°
                        </span>
                    </div>
                )}
            </div>

            {obj.kind === "sticky_note" && (
                <div className={styles.section}>
                    <span className={styles.sectionTitle}>Color</span>
                    <div className={styles.colorSwatches}>
                        {NOTE_COLORS.map((c) => {
                            const color = isDark ? c.dark : c.light;
                            const isActive =
                                currentColor === c.light ||
                                currentColor === c.dark;
                            return (
                                <button
                                    key={c.name}
                                    type="button"
                                    className={`${styles.swatch} ${isActive ? styles.activeSwatch : ""}`}
                                    style={{ background: color }}
                                    title={c.name}
                                    onClick={() =>
                                        handleColorChange(
                                            isDark ? c.dark : c.light,
                                        )
                                    }
                                />
                            );
                        })}
                    </div>
                </div>
            )}

            <div className={styles.section}>
                <span className={styles.sectionTitle}>Meta</span>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>ID</span>
                    <span className={styles.rowValue}>
                        {obj.id.slice(0, 8)}
                    </span>
                </div>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>Z</span>
                    <span className={styles.rowValue}>{obj.z_index}</span>
                </div>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>Ver</span>
                    <span className={styles.rowValue}>{obj.version}</span>
                </div>
            </div>
        </div>
    );
}

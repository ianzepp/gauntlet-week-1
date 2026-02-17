import { useState } from "react";
import { sendObjectCreate } from "../hooks/useFrameClient";
import type { BoardObject } from "../lib/types";
import { useBoardStore } from "../store/board";
import styles from "./ToolStrip.module.css";

interface ShapePreset {
    label: string;
    width: number;
    height: number;
    icon: React.ReactNode;
}

const SHAPE_PRESETS: ShapePreset[] = [
    {
        label: "Square",
        width: 120,
        height: 120,
        icon: (
            <svg viewBox="0 0 20 20">
                <rect x="3" y="3" width="14" height="14" />
            </svg>
        ),
    },
    {
        label: "Tall",
        width: 100,
        height: 160,
        icon: (
            <svg viewBox="0 0 20 20">
                <rect x="5" y="2" width="10" height="16" />
            </svg>
        ),
    },
    {
        label: "Wide",
        width: 200,
        height: 100,
        icon: (
            <svg viewBox="0 0 20 20">
                <rect x="2" y="5" width="16" height="10" />
            </svg>
        ),
    },
];

interface ColorPreset {
    label: string;
    value: string;
}

const COLOR_PRESETS: ColorPreset[] = [
    { label: "Red", value: "#D94B4B" },
    { label: "Blue", value: "#4B7DD9" },
    { label: "Green", value: "#4BAF6E" },
];

interface ToolStripProps {
    onClose: () => void;
}

export function ToolStrip({ onClose }: ToolStripProps) {
    const [shapeIndex, setShapeIndex] = useState(0);
    const [colorIndex, setColorIndex] = useState(0);

    const handleAdd = () => {
        const store = useBoardStore.getState();
        const { viewport, objects, boardId, viewportCenter } = store;

        const preset = SHAPE_PRESETS[shapeIndex];
        const color = COLOR_PRESETS[colorIndex];

        const newObj: BoardObject = {
            id: crypto.randomUUID(),
            board_id: boardId ?? "",
            kind: "rectangle",
            x: viewportCenter.x - preset.width / 2,
            y: viewportCenter.y - preset.height / 2,
            width: preset.width,
            height: preset.height,
            rotation: 0,
            z_index: objects.size,
            props: { color: color.value },
            created_by: "local",
            version: 1,
        };

        store.addObject(newObj);
        sendObjectCreate(newObj);
        store.setSelection(new Set([newObj.id]));
        onClose();
    };

    return (
        <div className={styles.strip}>
            <div className={styles.options}>
                {SHAPE_PRESETS.map((preset, i) => (
                    <button
                        key={preset.label}
                        type="button"
                        className={`${styles.option} ${i === shapeIndex ? styles.optionActive : ""}`}
                        onClick={() => setShapeIndex(i)}
                        title={preset.label}
                    >
                        {preset.icon}
                    </button>
                ))}
            </div>
            <div className={styles.divider} />
            <div className={styles.options}>
                {COLOR_PRESETS.map((color, i) => (
                    <button
                        key={color.label}
                        type="button"
                        className={`${styles.swatch} ${i === colorIndex ? styles.swatchActive : ""}`}
                        onClick={() => setColorIndex(i)}
                        title={color.label}
                    >
                        <span
                            className={styles.swatchColor}
                            style={{ background: color.value }}
                        />
                    </button>
                ))}
            </div>
            <div className={styles.divider} />
            <button
                type="button"
                className={styles.addButton}
                style={{ background: COLOR_PRESETS[colorIndex].value }}
                onClick={handleAdd}
            >
                Add
            </button>
        </div>
    );
}

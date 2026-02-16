import type { KonvaEventObject } from "konva/lib/Node";
import React, { useCallback } from "react";
import { Group, Ellipse as KonvaEllipse, Rect } from "react-konva";
import type { BoardObject } from "../lib/types";
import { useBoardStore } from "../store/board";

interface ShapeProps {
    object: BoardObject;
}

export const Shape = React.memo(function Shape({ object }: ShapeProps) {
    const selection = useBoardStore((s) => s.selection);
    const setSelection = useBoardStore((s) => s.setSelection);
    const updateObject = useBoardStore((s) => s.updateObject);

    const isSelected = selection.has(object.id);
    const strokeColor = isSelected ? "#2C2824" : "#8A8178";
    const strokeWidth = isSelected ? 2 : 1;

    const handleDragEnd = useCallback(
        (e: KonvaEventObject<DragEvent>) => {
            updateObject(object.id, {
                x: e.target.x(),
                y: e.target.y(),
            });
        },
        [object.id, updateObject],
    );

    const handleClick = useCallback(
        (e: KonvaEventObject<MouseEvent>) => {
            e.cancelBubble = true;
            setSelection(new Set([object.id]));
        },
        [object.id, setSelection],
    );

    return (
        <Group
            x={object.x}
            y={object.y}
            rotation={object.rotation}
            draggable
            onDragEnd={handleDragEnd}
            onClick={handleClick}
        >
            {object.kind === "rectangle" && (
                <Rect
                    width={object.width}
                    height={object.height}
                    stroke={strokeColor}
                    strokeWidth={strokeWidth}
                />
            )}
            {object.kind === "ellipse" && (
                <KonvaEllipse
                    x={object.width / 2}
                    y={object.height / 2}
                    radiusX={object.width / 2}
                    radiusY={object.height / 2}
                    stroke={strokeColor}
                    strokeWidth={strokeWidth}
                />
            )}
        </Group>
    );
});

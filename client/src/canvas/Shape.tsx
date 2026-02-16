import type { KonvaEventObject } from "konva/lib/Node";
import React, { useCallback, useEffect, useState } from "react";
import { Group, Ellipse as KonvaEllipse, Rect } from "react-konva";
import type { BoardObject } from "../lib/types";
import { useBoardStore } from "../store/board";

const COLORS = {
    light: { fill: "#f5f0e8", stroke: "#8a8178", selected: "#2c2824" },
    dark: { fill: "#2c2824", stroke: "#7a756d", selected: "#e8e0d2" },
};

function useIsDark() {
    const [isDark, setIsDark] = useState(() =>
        document.documentElement.classList.contains("dark-mode"),
    );
    useEffect(() => {
        const observer = new MutationObserver(() => {
            setIsDark(document.documentElement.classList.contains("dark-mode"));
        });
        observer.observe(document.documentElement, {
            attributes: true,
            attributeFilter: ["class"],
        });
        return () => observer.disconnect();
    }, []);
    return isDark;
}

interface ShapeProps {
    object: BoardObject;
}

export const Shape = React.memo(function Shape({ object }: ShapeProps) {
    const selection = useBoardStore((s) => s.selection);
    const setSelection = useBoardStore((s) => s.setSelection);
    const updateObject = useBoardStore((s) => s.updateObject);
    const isDark = useIsDark();

    const isSelected = selection.has(object.id);
    const palette = isDark ? COLORS.dark : COLORS.light;
    const fillColor = (object.props.color as string) ?? palette.fill;
    const strokeColor = isSelected ? palette.selected : palette.stroke;
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

    const handleTransformEnd = useCallback(
        (e: KonvaEventObject<Event>) => {
            const node = e.target;
            const scaleX = node.scaleX();
            const scaleY = node.scaleY();
            node.scaleX(1);
            node.scaleY(1);
            updateObject(object.id, {
                x: node.x(),
                y: node.y(),
                width: Math.max(5, object.width * scaleX),
                height: Math.max(5, object.height * scaleY),
                rotation: node.rotation(),
            });
        },
        [object.id, object.width, object.height, updateObject],
    );

    return (
        <Group
            name={`obj-${object.id}`}
            x={object.x}
            y={object.y}
            rotation={object.rotation}
            draggable={true}
            onDragEnd={handleDragEnd}
            onClick={handleClick}
            onTransformEnd={handleTransformEnd}
        >
            {object.kind === "rectangle" && (
                <Rect
                    width={object.width}
                    height={object.height}
                    fill={fillColor}
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
                    fill={fillColor}
                    stroke={strokeColor}
                    strokeWidth={strokeWidth}
                />
            )}
        </Group>
    );
});

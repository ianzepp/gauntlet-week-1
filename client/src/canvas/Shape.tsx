import type Konva from "konva";
import type { KonvaEventObject } from "konva/lib/Node";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { Ellipse as KonvaEllipse, Rect } from "react-konva";
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
    isSelected: boolean;
    onSelect: () => void;
    onShapeRef: (id: string, node: Konva.Node | null) => void;
}

export const Shape = function Shape({
    object,
    isSelected,
    onSelect,
    onShapeRef,
}: ShapeProps) {
    const updateObject = useBoardStore((s) => s.updateObject);
    const isDark = useIsDark();

    const palette = isDark ? COLORS.dark : COLORS.light;
    const fillColor = (object.props.color as string) ?? palette.fill;
    const strokeColor = isSelected ? palette.selected : palette.stroke;
    const strokeWidth = isSelected ? 2 : 1;

    const refCallback = useCallback(
        (node: Konva.Rect | Konva.Ellipse | null) => {
            onShapeRef(object.id, node);
        },
        [object.id, onShapeRef],
    );

    const handleDragEnd = useCallback(
        (e: KonvaEventObject<DragEvent>) => {
            updateObject(object.id, {
                x: e.target.x(),
                y: e.target.y(),
            });
        },
        [object.id, updateObject],
    );

    const handleTransformEnd = useCallback(
        (e: KonvaEventObject<Event>) => {
            const node = e.target;
            const scaleX = node.scaleX();
            const scaleY = node.scaleY();
            node.scaleX(1);
            node.scaleY(1);

            if (object.kind === "ellipse") {
                const ellipse = node as Konva.Ellipse;
                updateObject(object.id, {
                    x: node.x() - ellipse.radiusX(),
                    y: node.y() - ellipse.radiusY(),
                    width: Math.max(5, ellipse.radiusX() * 2),
                    height: Math.max(5, ellipse.radiusY() * 2),
                    rotation: node.rotation(),
                });
            } else {
                updateObject(object.id, {
                    x: node.x(),
                    y: node.y(),
                    width: Math.max(5, node.width() * scaleX),
                    height: Math.max(5, node.height() * scaleY),
                    rotation: node.rotation(),
                });
            }
        },
        [object.id, object.kind, updateObject],
    );

    if (object.kind === "ellipse") {
        return (
            <KonvaEllipse
                ref={refCallback}
                x={object.x + object.width / 2}
                y={object.y + object.height / 2}
                radiusX={object.width / 2}
                radiusY={object.height / 2}
                rotation={object.rotation}
                draggable
                onClick={onSelect}
                onTap={onSelect}
                onDragEnd={handleDragEnd}
                onTransformEnd={handleTransformEnd}
                fill={fillColor}
                stroke={strokeColor}
                strokeWidth={strokeWidth}
            />
        );
    }

    return (
        <Rect
            ref={refCallback}
            x={object.x}
            y={object.y}
            width={object.width}
            height={object.height}
            rotation={object.rotation}
            draggable
            onClick={onSelect}
            onTap={onSelect}
            onDragEnd={handleDragEnd}
            onTransformEnd={handleTransformEnd}
            fill={fillColor}
            stroke={strokeColor}
            strokeWidth={strokeWidth}
        />
    );
};

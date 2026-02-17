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

function sendObjectUpdate(objectId: string, fields: Record<string, unknown>) {
    const store = useBoardStore.getState();
    const client = store.frameClient;
    const boardId = store.boardId;
    if (!client || !boardId) return;

    client.send({
        id: crypto.randomUUID(),
        parent_id: null,
        ts: Date.now(),
        board_id: boardId,
        from: null,
        syscall: "object:update",
        status: "request",
        data: { id: objectId, ...fields },
    });
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
            const x = e.target.x();
            const y = e.target.y();
            updateObject(object.id, { x, y });
            sendObjectUpdate(object.id, { x, y, version: object.version });
        },
        [object.id, object.version, updateObject],
    );

    const handleTransformEnd = useCallback(
        (e: KonvaEventObject<Event>) => {
            const node = e.target;
            const scaleX = node.scaleX();
            const scaleY = node.scaleY();
            node.scaleX(1);
            node.scaleY(1);

            let updates: Partial<BoardObject>;
            if (object.kind === "ellipse") {
                const ellipse = node as Konva.Ellipse;
                const radiusX = Math.max(2.5, ellipse.radiusX() * scaleX);
                const radiusY = Math.max(2.5, ellipse.radiusY() * scaleY);
                updates = {
                    x: node.x() - radiusX,
                    y: node.y() - radiusY,
                    width: Math.max(5, radiusX * 2),
                    height: Math.max(5, radiusY * 2),
                    rotation: node.rotation(),
                };
            } else {
                updates = {
                    x: node.x(),
                    y: node.y(),
                    width: Math.max(5, node.width() * scaleX),
                    height: Math.max(5, node.height() * scaleY),
                    rotation: node.rotation(),
                };
            }
            updateObject(object.id, updates);
            sendObjectUpdate(object.id, {
                ...updates,
                version: object.version,
            });
        },
        [object.id, object.kind, object.version, updateObject],
    );

    if (object.kind === "ellipse") {
        return (
            <KonvaEllipse
                ref={refCallback}
                x={object.x + object.width / 2}
                y={object.y + object.height / 2}
                radiusX={object.width / 2}
                radiusY={object.height / 2}
                name="board-object"
                objectId={object.id}
                rotation={object.rotation}
                draggable
                onMouseDown={onSelect}
                onClick={onSelect}
                onTap={onSelect}
                onDragStart={onSelect}
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
            name="board-object"
            objectId={object.id}
            rotation={object.rotation}
            draggable
            onMouseDown={onSelect}
            onClick={onSelect}
            onTap={onSelect}
            onDragStart={onSelect}
            onDragEnd={handleDragEnd}
            onTransformEnd={handleTransformEnd}
            fill={fillColor}
            stroke={strokeColor}
            strokeWidth={strokeWidth}
        />
    );
};

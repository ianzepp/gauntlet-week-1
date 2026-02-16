import type Konva from "konva";
import type { KonvaEventObject } from "konva/lib/Node";
import React, { useCallback, useRef, useState } from "react";
import { Group, Rect, Text } from "react-konva";
import type { BoardObject } from "../lib/types";
import { useBoardStore } from "../store/board";
import { TextEditor } from "./TextEditor";

interface StickyNoteProps {
    object: BoardObject;
    isSelected: boolean;
    onSelect: () => void;
    onShapeRef: (id: string, node: Konva.Node | null) => void;
}

function darkenColor(hex: string): string {
    const r = Math.max(0, Number.parseInt(hex.slice(1, 3), 16) - 20);
    const g = Math.max(0, Number.parseInt(hex.slice(3, 5), 16) - 20);
    const b = Math.max(0, Number.parseInt(hex.slice(5, 7), 16) - 20);
    return `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
}

export const StickyNote = function StickyNote({
    object,
    isSelected,
    onSelect,
    onShapeRef,
}: StickyNoteProps) {
    const [editing, setEditing] = useState(false);

    const updateObject = useBoardStore((s) => s.updateObject);
    const viewport = useBoardStore((s) => s.viewport);

    const color = (object.props.color as string) ?? "#F5F0E8";
    const text = (object.props.text as string) ?? "";
    const borderColor = darkenColor(color);

    const refCallback = useCallback(
        (node: Konva.Group | null) => {
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

    const handleDblClick = useCallback(() => {
        setEditing(true);
    }, []);

    const handleTextSave = useCallback(
        (newText: string) => {
            updateObject(object.id, {
                props: { ...object.props, text: newText },
            });
            setEditing(false);
        },
        [object.id, object.props, updateObject],
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

    const screenX = object.x * viewport.scale + viewport.x;
    const screenY = object.y * viewport.scale + viewport.y;

    return (
        <Group
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
            onDblClick={handleDblClick}
            onTransformEnd={handleTransformEnd}
        >
            <Rect
                width={object.width}
                height={object.height}
                fill={color}
                stroke={isSelected ? "#2C2824" : borderColor}
                strokeWidth={isSelected ? 2 : 1}
            />
            <Text
                text={text}
                width={object.width - 16}
                height={object.height - 16}
                x={8}
                y={8}
                fontFamily="Caveat"
                fontSize={20}
                fill="#2C2824"
                wrap="word"
                listening={false}
            />
            {editing && (
                <TextEditor
                    x={screenX}
                    y={screenY}
                    width={object.width * viewport.scale}
                    height={object.height * viewport.scale}
                    text={text}
                    onSave={handleTextSave}
                    onCancel={() => setEditing(false)}
                />
            )}
        </Group>
    );
};

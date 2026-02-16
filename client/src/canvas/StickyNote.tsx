import type Konva from "konva";
import type { KonvaEventObject } from "konva/lib/Node";
import React, { useCallback, useRef, useState } from "react";
import { Group, Rect, Text } from "react-konva";
import type { BoardObject } from "../lib/types";
import { useBoardStore } from "../store/board";
import { TextEditor } from "./TextEditor";

interface StickyNoteProps {
    object: BoardObject;
}

function darkenColor(hex: string): string {
    const r = Math.max(0, Number.parseInt(hex.slice(1, 3), 16) - 20);
    const g = Math.max(0, Number.parseInt(hex.slice(3, 5), 16) - 20);
    const b = Math.max(0, Number.parseInt(hex.slice(5, 7), 16) - 20);
    return `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
}

export const StickyNote = React.memo(function StickyNote({
    object,
}: StickyNoteProps) {
    const [editing, setEditing] = useState(false);
    const groupRef = useRef<Konva.Group>(null);

    const selection = useBoardStore((s) => s.selection);
    const setSelection = useBoardStore((s) => s.setSelection);
    const updateObject = useBoardStore((s) => s.updateObject);
    const viewport = useBoardStore((s) => s.viewport);

    const isSelected = selection.has(object.id);
    const color = (object.props.color as string) ?? "#F5F0E8";
    const text = (object.props.text as string) ?? "";
    const borderColor = darkenColor(color);

    const handleDragEnd = useCallback(
        (e: KonvaEventObject<DragEvent>) => {
            updateObject(object.id, {
                x: e.target.x(),
                y: e.target.y(),
                rotation: 0,
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

    const screenX = object.x * viewport.scale + viewport.x;
    const screenY = object.y * viewport.scale + viewport.y;

    return (
        <>
            <Group
                ref={groupRef}
                x={object.x}
                y={object.y}
                rotation={object.rotation}
                draggable
                onDragEnd={handleDragEnd}
                onClick={handleClick}
                onDblClick={handleDblClick}
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
            </Group>
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
        </>
    );
});

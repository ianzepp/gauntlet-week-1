import type Konva from "konva";
import type { KonvaEventObject } from "konva/lib/Node";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Circle, Group, Layer, Line, Rect, Stage, Text } from "react-konva";
import { useCanvasSize } from "../hooks/useCanvasSize";
import type { Presence } from "../lib/types";
import { useBoardStore } from "../store/board";

const GRID_SIZE = 20;
const GRID_MAJOR = 5;
const CURSOR_SEND_INTERVAL_MS = 50;
const CURSOR_SEND_MIN_DELTA = 0.5;

function GridLines({
    width,
    height,
    viewport,
}: {
    width: number;
    height: number;
    viewport: { x: number; y: number; scale: number };
}) {
    const { x: ox, y: oy, scale } = viewport;

    const startX = Math.floor(-ox / scale / GRID_SIZE) * GRID_SIZE;
    const endX = startX + Math.ceil(width / scale / GRID_SIZE + 1) * GRID_SIZE;
    const startY = Math.floor(-oy / scale / GRID_SIZE) * GRID_SIZE;
    const endY = startY + Math.ceil(height / scale / GRID_SIZE + 1) * GRID_SIZE;

    const lines: React.JSX.Element[] = [];

    for (let x = startX; x <= endX; x += GRID_SIZE) {
        const major = x % (GRID_SIZE * GRID_MAJOR) === 0;
        lines.push(
            <Line
                key={`v${x}`}
                points={[x, startY, x, endY]}
                stroke={major ? "var(--grid-major, #c8c0b4)" : "var(--grid-minor, #d4cfc6)"}
                strokeWidth={1 / scale}
                opacity={major ? 0.4 : 0.2}
                listening={false}
            />,
        );
    }

    for (let y = startY; y <= endY; y += GRID_SIZE) {
        const major = y % (GRID_SIZE * GRID_MAJOR) === 0;
        lines.push(
            <Line
                key={`h${y}`}
                points={[startX, y, endX, y]}
                stroke={major ? "var(--grid-major, #c8c0b4)" : "var(--grid-minor, #d4cfc6)"}
                strokeWidth={1 / scale}
                opacity={major ? 0.4 : 0.2}
                listening={false}
            />,
        );
    }

    // Origin crosshair — slightly more visible than the grid
    lines.push(
        <Line
            key="origin-h"
            points={[startX, 0, endX, 0]}
            stroke="#a09888"
            strokeWidth={1 / scale}
            opacity={0.6}
            listening={false}
        />,
        <Line
            key="origin-v"
            points={[0, startY, 0, endY]}
            stroke="#a09888"
            strokeWidth={1 / scale}
            opacity={0.6}
            listening={false}
        />,
    );

    return <>{lines}</>;
}

export function Canvas() {
    const { width, height } = useCanvasSize();
    const stageRef = useRef<Konva.Stage>(null);
    const editorRef = useRef<HTMLDivElement | null>(null);
    const centeredRef = useRef(false);
    const isDraggingRef = useRef(false);
    const dragStartRef = useRef({ x: 0, y: 0 });
    const objectDragRef = useRef<{
        id: string;
        pointerX: number;
        pointerY: number;
        objectX: number;
        objectY: number;
    } | null>(null);
    const lastCursorSendTsRef = useRef(0);
    const lastCursorSentPosRef = useRef<{ x: number; y: number } | null>(null);

    const boardId = useBoardStore((s) => s.boardId);
    const user = useBoardStore((s) => s.user);
    const presence = useBoardStore((s) => s.presence);
    const viewport = useBoardStore((s) => s.viewport);
    const setViewport = useBoardStore((s) => s.setViewport);
    const setCursorPosition = useBoardStore((s) => s.setCursorPosition);
    const setViewportCenter = useBoardStore((s) => s.setViewportCenter);
    const setSelection = useBoardStore((s) => s.setSelection);
    const updateObject = useBoardStore((s) => s.updateObject);
    const frameClient = useBoardStore((s) => s.frameClient);
    const objects = useBoardStore((s) => s.objects);
    const selection = useBoardStore((s) => s.selection);
    const [editingObjectId, setEditingObjectId] = useState<string | null>(null);
    const [editingText, setEditingText] = useState("");

    const objectList = useMemo(
        () =>
            Array.from(objects.values()).sort((a, b) => {
                if (a.z_index !== b.z_index) return a.z_index - b.z_index;
                return a.id.localeCompare(b.id);
            }),
        [objects],
    );
    const editingObject = editingObjectId ? objects.get(editingObjectId) ?? null : null;
    const remoteCursors = useMemo(
        () =>
            Array.from(presence.values()).filter(
                (p): p is Presence & { cursor: { x: number; y: number } } => p.cursor !== null,
            ),
        [presence],
    );

    const findTopRectangleAtPoint = useCallback(
        (canvasX: number, canvasY: number) =>
            [...objectList].reverse().find((obj) => {
                if (obj.kind !== "rectangle" || obj.width == null || obj.height == null) {
                    return false;
                }
                return (
                    canvasX >= obj.x &&
                    canvasX <= obj.x + obj.width &&
                    canvasY >= obj.y &&
                    canvasY <= obj.y + obj.height
                );
            }),
        [objectList],
    );

    const finishEditing = useCallback(
        (save: boolean) => {
            if (!editingObjectId) return;
            const objectId = editingObjectId;
            const obj = objects.get(objectId);
            setEditingObjectId(null);

            if (!save || !obj) return;

            const previousText = (obj.props.text as string) ?? "";
            if (previousText === editingText) return;

            const nextProps = { ...obj.props, text: editingText };
            updateObject(objectId, { props: nextProps });

            if (!frameClient) return;
            frameClient.send({
                id: crypto.randomUUID(),
                parent_id: null,
                ts: Date.now(),
                board_id: obj.board_id,
                from: null,
                syscall: "object:update",
                status: "request",
                data: {
                    id: objectId,
                    props: nextProps,
                    version: obj.version,
                },
            });
        },
        [editingObjectId, editingText, frameClient, objects, updateObject],
    );

    const beginEditing = useCallback(
        (objectId: string) => {
            const obj = objects.get(objectId);
            if (!obj) return;
            const text = (obj.props.text as string) ?? "";
            setSelection(new Set([objectId]));
            setEditingObjectId(objectId);
            setEditingText(text);
            objectDragRef.current = null;
            isDraggingRef.current = false;
        },
        [objects, setSelection],
    );

    // Center viewport so canvas origin (0,0) is at screen center on first mount
    useEffect(() => {
        if (!centeredRef.current && width > 0 && height > 0) {
            centeredRef.current = true;
            setViewport({ x: width / 2, y: height / 2 });
        }
    }, [width, height, setViewport]);

    useEffect(() => {
        if (!editingObjectId) return;
        const node = editorRef.current;
        if (!node) return;
        node.textContent = editingText;
        node.focus();
        const selection = window.getSelection();
        if (!selection) return;
        const range = document.createRange();
        range.selectNodeContents(node);
        range.collapse(false);
        selection.removeAllRanges();
        selection.addRange(range);
    }, [editingObjectId]);

    useEffect(() => {
        if (editingObjectId && !objects.has(editingObjectId)) {
            setEditingObjectId(null);
        }
    }, [editingObjectId, objects]);

    // Keep viewport center in sync for status bar
    useEffect(() => {
        const cx = Math.round((-viewport.x + width / 2) / viewport.scale);
        const cy = Math.round((-viewport.y + height / 2) / viewport.scale);
        setViewportCenter({ x: cx, y: cy });
    }, [viewport, width, height, setViewportCenter]);

    // Pan: wheel/trackpad scroll; Zoom: ctrl/cmd + wheel
    const handleWheel = useCallback(
        (e: KonvaEventObject<WheelEvent>) => {
            e.evt.preventDefault();
            const stage = stageRef.current;
            if (!stage) return;

            if (e.evt.ctrlKey || e.evt.metaKey) {
                const pointer = stage.getPointerPosition();
                if (!pointer) return;

                const oldScale = viewport.scale;
                const scaleBy = 1.05;
                const newScale =
                    e.evt.deltaY < 0 ? oldScale * scaleBy : oldScale / scaleBy;
                const clampedScale = Math.max(0.1, Math.min(5, newScale));

                const mousePointTo = {
                    x: (pointer.x - viewport.x) / oldScale,
                    y: (pointer.y - viewport.y) / oldScale,
                };

                setViewport({
                    scale: clampedScale,
                    x: pointer.x - mousePointTo.x * clampedScale,
                    y: pointer.y - mousePointTo.y * clampedScale,
                });
            } else {
                setViewport({
                    x: viewport.x - e.evt.deltaX,
                    y: viewport.y - e.evt.deltaY,
                });
            }
        },
        [viewport, setViewport],
    );

    const commitDraggedObject = useCallback(() => {
        const drag = objectDragRef.current;
        if (!drag) return;
        objectDragRef.current = null;

        const current = useBoardStore.getState().objects.get(drag.id);
        if (!frameClient || !current) return;
        frameClient.send({
            id: crypto.randomUUID(),
            parent_id: null,
            ts: Date.now(),
            board_id: current.board_id,
            from: null,
            syscall: "object:update",
            status: "request",
            data: {
                id: current.id,
                x: current.x,
                y: current.y,
                version: current.version,
            },
        });
    }, [frameClient]);

    const handleMouseDown = useCallback(
        (_e: KonvaEventObject<MouseEvent>) => {
            if (editingObjectId) return;
            const stage = stageRef.current;
            const pointer = stage?.getPointerPosition();
            if (!stage || !pointer) return;

            const canvasX = (pointer.x - viewport.x) / viewport.scale;
            const canvasY = (pointer.y - viewport.y) / viewport.scale;

            const hit = findTopRectangleAtPoint(canvasX, canvasY);

            if (hit) {
                setSelection(new Set([hit.id]));
                objectDragRef.current = {
                    id: hit.id,
                    pointerX: canvasX,
                    pointerY: canvasY,
                    objectX: hit.x,
                    objectY: hit.y,
                };
                isDraggingRef.current = false;
                const container = stage.container();
                container.style.cursor = "move";
                return;
            }

            setSelection(new Set());
            isDraggingRef.current = true;
            dragStartRef.current = { x: pointer.x, y: pointer.y };
            const container = stage.container();
            container.style.cursor = "grabbing";
        },
        [editingObjectId, findTopRectangleAtPoint, setSelection, viewport],
    );

    const handleMouseMove = useCallback(
        (e: KonvaEventObject<MouseEvent>) => {
            const stage = stageRef.current;
            if (!stage) return;

            const pointer = stage.getPointerPosition();
            if (!pointer) return;
            const canvasX = (pointer.x - viewport.x) / viewport.scale;
            const canvasY = (pointer.y - viewport.y) / viewport.scale;

            if (objectDragRef.current) {
                const drag = objectDragRef.current;
                updateObject(drag.id, {
                    x: drag.objectX + (canvasX - drag.pointerX),
                    y: drag.objectY + (canvasY - drag.pointerY),
                });
            } else if (isDraggingRef.current) {
                const dx = e.evt.clientX - dragStartRef.current.x;
                const dy = e.evt.clientY - dragStartRef.current.y;
                dragStartRef.current = { x: e.evt.clientX, y: e.evt.clientY };
                const v = useBoardStore.getState().viewport;
                setViewport({ x: v.x + dx, y: v.y + dy });
            }

            setCursorPosition({ x: Math.round(canvasX), y: Math.round(canvasY) });

            if (frameClient && boardId) {
                const now = Date.now();
                const last = lastCursorSentPosRef.current;
                const movedEnough =
                    !last ||
                    Math.abs(canvasX - last.x) >= CURSOR_SEND_MIN_DELTA ||
                    Math.abs(canvasY - last.y) >= CURSOR_SEND_MIN_DELTA;
                if (movedEnough && now - lastCursorSendTsRef.current >= CURSOR_SEND_INTERVAL_MS) {
                    frameClient.send({
                        id: crypto.randomUUID(),
                        parent_id: null,
                        ts: now,
                        board_id: boardId,
                        from: null,
                        syscall: "cursor:moved",
                        status: "request",
                        data: {
                            x: canvasX,
                            y: canvasY,
                            name: user?.name ?? "Anonymous",
                            color: user?.color ?? "#6366f1",
                        },
                    });
                    lastCursorSendTsRef.current = now;
                    lastCursorSentPosRef.current = { x: canvasX, y: canvasY };
                }
            }
        },
        [boardId, frameClient, setCursorPosition, setViewport, updateObject, user, viewport],
    );

    const handleMouseUp = useCallback(() => {
        commitDraggedObject();
        isDraggingRef.current = false;
        const container = stageRef.current?.container();
        if (container) container.style.cursor = "";
    }, [commitDraggedObject]);

    const handleMouseLeave = useCallback(() => {
        commitDraggedObject();
        isDraggingRef.current = false;
        const container = stageRef.current?.container();
        if (container) container.style.cursor = "";
        setCursorPosition(null);
        lastCursorSentPosRef.current = null;
    }, [commitDraggedObject, setCursorPosition]);

    const handleDoubleClick = useCallback(
        (_e: KonvaEventObject<MouseEvent>) => {
            const stage = stageRef.current;
            const pointer = stage?.getPointerPosition();
            if (!pointer) return;
            const canvasX = (pointer.x - viewport.x) / viewport.scale;
            const canvasY = (pointer.y - viewport.y) / viewport.scale;
            const hit = findTopRectangleAtPoint(canvasX, canvasY);
            if (hit) beginEditing(hit.id);
        },
        [beginEditing, findTopRectangleAtPoint, viewport],
    );

    return (
        <div
            style={{
                position: "fixed",
                inset: 0,
                zIndex: 0,
                background: "var(--canvas-bg)",
            }}
        >
            <Stage
                ref={stageRef}
                width={width}
                height={height}
                scaleX={viewport.scale}
                scaleY={viewport.scale}
                x={viewport.x}
                y={viewport.y}
                onWheel={handleWheel}
                onMouseDown={handleMouseDown}
                onMouseMove={handleMouseMove}
                onMouseUp={handleMouseUp}
                onMouseLeave={handleMouseLeave}
                onDblClick={handleDoubleClick}
            >
                <Layer listening={false}>
                    <GridLines
                        width={width}
                        height={height}
                        viewport={viewport}
                    />
                    <Circle x={0} y={0} radius={5} fill="red" listening={false} />
                </Layer>
                <Layer>
                    {objectList.map((obj) => {
                        if (obj.kind === "rectangle") {
                            const isSelected = selection.has(obj.id);
                            const fill = (obj.props.color as string) ?? "#D94B4B";
                            return (
                                <Rect
                                    id={obj.id}
                                    key={obj.localKey ?? obj.id}
                                    x={obj.x}
                                    y={obj.y}
                                    width={obj.width ?? 0}
                                    height={obj.height ?? 0}
                                    rotation={obj.rotation}
                                    fill={fill}
                                    stroke={isSelected ? "#fff" : fill}
                                    strokeWidth={isSelected ? 2 : 0}
                                    listening={false}
                                />
                            );
                        }
                        return null;
                    })}
                    {objectList.map((obj) => {
                        if (obj.kind !== "rectangle") return null;
                        const text = (obj.props.text as string) ?? "";
                        if (!text) return null;
                        const widthPx = Math.max(obj.width ?? 0, 0);
                        const heightPx = Math.max(obj.height ?? 0, 0);
                        if (widthPx === 0 || heightPx === 0) return null;
                        return (
                            <Text
                                key={`text-${obj.localKey ?? obj.id}`}
                                x={obj.x}
                                y={obj.y}
                                width={widthPx}
                                height={heightPx}
                                rotation={obj.rotation}
                                text={text}
                                fontFamily={"Caveat, Patrick Hand, Comic Sans MS, cursive"}
                                fontSize={13}
                                align="center"
                                verticalAlign="middle"
                                fill="#1F1A17"
                                listening={false}
                            />
                        );
                    })}
                </Layer>
                <Layer listening={false}>
                    {remoteCursors.map((presenceItem) => {
                        const label = presenceItem.name || "Anonymous";
                        const labelWidth = Math.max(54, Math.min(190, label.length * 7 + 16));
                        return (
                            <Group
                                key={`cursor-${presenceItem.user_id}`}
                                x={presenceItem.cursor.x}
                                y={presenceItem.cursor.y}
                                scaleX={1 / viewport.scale}
                                scaleY={1 / viewport.scale}
                                listening={false}
                            >
                                <Line
                                    points={[0, 0, 0, 18, 5, 13, 9, 24, 13, 22, 9, 12, 18, 12]}
                                    closed
                                    fill={presenceItem.color}
                                    stroke="#ffffff"
                                    strokeWidth={1}
                                    listening={false}
                                />
                                <Rect
                                    x={18}
                                    y={-11}
                                    width={labelWidth}
                                    height={22}
                                    cornerRadius={7}
                                    fill="rgba(17,17,17,0.82)"
                                    stroke={presenceItem.color}
                                    strokeWidth={1}
                                    listening={false}
                                />
                                <Text
                                    x={25}
                                    y={-6}
                                    width={labelWidth - 10}
                                    height={12}
                                    text={label}
                                    fontFamily="monospace"
                                    fontSize={12}
                                    fill="#ffffff"
                                    listening={false}
                                />
                            </Group>
                        );
                    })}
                </Layer>
            </Stage>
            {editingObject && (
                <div
                    style={{
                        position: "absolute",
                        left: viewport.x + editingObject.x * viewport.scale,
                        top: viewport.y + editingObject.y * viewport.scale,
                        width: (editingObject.width ?? 0) * viewport.scale,
                        height: (editingObject.height ?? 0) * viewport.scale,
                        transform: `rotate(${editingObject.rotation}deg)`,
                        transformOrigin: "top left",
                        zIndex: 2,
                        pointerEvents: "auto",
                        border: "1px solid rgba(255,255,255,0.8)",
                        background: "rgba(255,255,255,0.9)",
                        padding: 8,
                        color: "#1F1A17",
                        fontSize: 16 * viewport.scale,
                        lineHeight: 1.2,
                        whiteSpace: "pre-wrap",
                        overflow: "hidden",
                        outline: "none",
                    }}
                    ref={editorRef}
                    contentEditable
                    suppressContentEditableWarning
                    onInput={(e) => setEditingText(e.currentTarget.textContent ?? "")}
                    onBlur={() => finishEditing(true)}
                    onKeyDown={(e) => {
                        if (e.key === "Escape") {
                            e.preventDefault();
                            finishEditing(false);
                        } else if (e.key === "Enter" && !e.shiftKey) {
                            e.preventDefault();
                            finishEditing(true);
                        }
                    }}
                />
            )}
        </div>
    );
}

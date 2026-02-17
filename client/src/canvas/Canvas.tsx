import type Konva from "konva";
import type { KonvaEventObject } from "konva/lib/Node";
import React, { useCallback, useEffect, useMemo, useRef } from "react";
import { Circle, Layer, Line, Rect, Stage } from "react-konva";
import { useCanvasSize } from "../hooks/useCanvasSize";
import { useBoardStore } from "../store/board";

const GRID_SIZE = 20;
const GRID_MAJOR = 5;

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
    const centeredRef = useRef(false);
    const isDraggingRef = useRef(false);
    const dragStartRef = useRef({ x: 0, y: 0 });

    const viewport = useBoardStore((s) => s.viewport);
    const setViewport = useBoardStore((s) => s.setViewport);
    const setCursorPosition = useBoardStore((s) => s.setCursorPosition);
    const setViewportCenter = useBoardStore((s) => s.setViewportCenter);
    const setSelection = useBoardStore((s) => s.setSelection);
    const updateObject = useBoardStore((s) => s.updateObject);
    const frameClient = useBoardStore((s) => s.frameClient);
    const objects = useBoardStore((s) => s.objects);
    const selection = useBoardStore((s) => s.selection);

    const objectList = useMemo(
        () =>
            Array.from(objects.values()).sort((a, b) => {
                if (a.z_index !== b.z_index) return a.z_index - b.z_index;
                return a.id.localeCompare(b.id);
            }),
        [objects],
    );

    // Center viewport so canvas origin (0,0) is at screen center on first mount
    useEffect(() => {
        if (!centeredRef.current && width > 0 && height > 0) {
            centeredRef.current = true;
            setViewport({ x: width / 2, y: height / 2 });
        }
    }, [width, height, setViewport]);

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

    const handleMouseDown = useCallback(
        (e: KonvaEventObject<MouseEvent>) => {
            // Only pan when clicking empty canvas
            if (e.target !== e.target.getStage()) return;
            setSelection(new Set());
            isDraggingRef.current = true;
            dragStartRef.current = { x: e.evt.clientX, y: e.evt.clientY };
            const container = stageRef.current?.container();
            if (container) container.style.cursor = "grabbing";
        },
        [setSelection],
    );

    const handleMouseMove = useCallback(
        (e: KonvaEventObject<MouseEvent>) => {
            const stage = stageRef.current;
            if (!stage) return;

            if (isDraggingRef.current) {
                const dx = e.evt.clientX - dragStartRef.current.x;
                const dy = e.evt.clientY - dragStartRef.current.y;
                dragStartRef.current = { x: e.evt.clientX, y: e.evt.clientY };
                const v = useBoardStore.getState().viewport;
                setViewport({ x: v.x + dx, y: v.y + dy });
            }

            const pointer = stage.getPointerPosition();
            if (!pointer) return;
            const canvasX = Math.round((pointer.x - viewport.x) / viewport.scale);
            const canvasY = Math.round((pointer.y - viewport.y) / viewport.scale);
            setCursorPosition({ x: canvasX, y: canvasY });
        },
        [viewport, setCursorPosition, setViewport],
    );

    const handleMouseUp = useCallback(() => {
        isDraggingRef.current = false;
        const container = stageRef.current?.container();
        if (container) container.style.cursor = "";
    }, []);

    const handleMouseLeave = useCallback(() => {
        isDraggingRef.current = false;
        const container = stageRef.current?.container();
        if (container) container.style.cursor = "";
        setCursorPosition(null);
    }, [setCursorPosition]);

    const handleObjectMouseDown = useCallback(
        (e: KonvaEventObject<MouseEvent>, objectId: string) => {
            e.cancelBubble = true;
            const stage = stageRef.current;
            const pointer = stage?.getPointerPosition();
            if (!stage || !pointer) {
                setSelection(new Set([objectId]));
                return;
            }

            const overlappingIds = stage
                .getAllIntersections(pointer)
                .map((node) => node.id())
                .filter((id) => id && objects.has(id));

            if (overlappingIds.length <= 1) {
                setSelection(new Set([objectId]));
                return;
            }

            const currentSelected = selection.size === 1
                ? Array.from(selection)[0]
                : null;
            if (currentSelected !== objectId) {
                setSelection(new Set([objectId]));
                return;
            }
            const currentIndex = currentSelected
                ? overlappingIds.indexOf(currentSelected)
                : -1;
            const nextId = currentIndex >= 0
                ? overlappingIds[(currentIndex + 1) % overlappingIds.length]
                : overlappingIds[0];

            setSelection(new Set([nextId]));
        },
        [objects, selection, setSelection],
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
                                    draggable={isSelected}
                                    onMouseDown={(e) => handleObjectMouseDown(e, obj.id)}
                                    onDragMove={(e) => {
                                        updateObject(obj.id, {
                                            x: e.target.x(),
                                            y: e.target.y(),
                                        });
                                    }}
                                    onDragEnd={(e) => {
                                        const x = e.target.x();
                                        const y = e.target.y();
                                        updateObject(obj.id, { x, y });

                                        const current = useBoardStore.getState().objects.get(obj.id);
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
                                                id: obj.id,
                                                x,
                                                y,
                                                version: current.version,
                                            },
                                        });
                                    }}
                                />
                            );
                        }
                        return null;
                    })}
                </Layer>
            </Stage>
        </div>
    );
}

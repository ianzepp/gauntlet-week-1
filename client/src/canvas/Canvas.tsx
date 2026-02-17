import type Konva from "konva";
import type { KonvaEventObject } from "konva/lib/Node";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Arrow, Circle, Group, Layer, Line, Stage, Text, Transformer } from "react-konva";
import { useCanvasSize } from "../hooks/useCanvasSize";
import type { BoardObject, Frame } from "../lib/types";
import { useBoardStore } from "../store/board";
import { Shape } from "./Shape";
import { StickyNote } from "./StickyNote";

const NOTE_COLORS = [
    "#F5F0E8",
    "#B8C5B0",
    "#C4A882",
    "#9AA3AD",
    "#C2A8A0",
    "#C9B97A",
    "#A8A298",
    "#8B9E7E",
];

const GRID_SIZE = 20;
const GRID_MAJOR = 5;
const CURSOR_THROTTLE_MS = 50;

function GridLines({
    width,
    height,
    viewport,
    isDark,
}: {
    width: number;
    height: number;
    viewport: { x: number; y: number; scale: number };
    isDark: boolean;
}) {
    const lines = useMemo(() => {
        const result: { points: number[]; major: boolean }[] = [];
        const { x: ox, y: oy, scale } = viewport;

        const startX = Math.floor(-ox / scale / GRID_SIZE) * GRID_SIZE;
        const endX =
            startX + Math.ceil(width / scale / GRID_SIZE + 1) * GRID_SIZE;
        const startY = Math.floor(-oy / scale / GRID_SIZE) * GRID_SIZE;
        const endY =
            startY + Math.ceil(height / scale / GRID_SIZE + 1) * GRID_SIZE;

        for (let x = startX; x <= endX; x += GRID_SIZE) {
            const major = x % (GRID_SIZE * GRID_MAJOR) === 0;
            result.push({ points: [x, startY, x, endY], major });
        }
        for (let y = startY; y <= endY; y += GRID_SIZE) {
            const major = y % (GRID_SIZE * GRID_MAJOR) === 0;
            result.push({ points: [startX, y, endX, y], major });
        }
        return result;
    }, [width, height, viewport]);

    return (
        <>
            {lines.map((line) => (
                <Line
                    key={line.points.join(",")}
                    points={line.points}
                    stroke={isDark ? (line.major ? "#3a3632" : "#2a2724") : (line.major ? "#c8c0b4" : "#d4cfc6")}
                    strokeWidth={1 / viewport.scale}
                    opacity={line.major ? 0.4 : 0.2}
                    listening={false}
                />
            ))}
        </>
    );
}

function RemoteCursor({ name, color, x, y, scale }: { name: string; color: string; x: number; y: number; scale: number }) {
    const fontSize = 11 / scale;
    const arrowSize = 8 / scale;
    return (
        <Group x={x} y={y} listening={false}>
            <Arrow
                points={[0, 0, arrowSize, arrowSize * 1.5]}
                fill={color}
                stroke={color}
                strokeWidth={1 / scale}
                pointerLength={arrowSize * 0.6}
                pointerWidth={arrowSize * 0.4}
            />
            <Circle
                x={0}
                y={0}
                radius={3 / scale}
                fill={color}
            />
            <Text
                x={arrowSize * 1.2}
                y={arrowSize * 1.8}
                text={name}
                fontSize={fontSize}
                fill="#fff"
                padding={2 / scale}
                listening={false}
            />
            <Text
                x={arrowSize * 1.2 - 1 / scale}
                y={arrowSize * 1.8 - 1 / scale}
                text={name}
                fontSize={fontSize}
                fill={color}
                padding={2 / scale}
                listening={false}
            />
        </Group>
    );
}

export function Canvas() {
    const { width, height } = useCanvasSize();
    const stageRef = useRef<Konva.Stage>(null);
    const trRef = useRef<Konva.Transformer>(null);
    const nodeMapRef = useRef<Map<string, Konva.Node>>(new Map());
    const lastCursorSendRef = useRef(0);

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

    const viewport = useBoardStore((s) => s.viewport);
    const objects = useBoardStore((s) => s.objects);
    const activeTool = useBoardStore((s) => s.activeTool);
    const setViewport = useBoardStore((s) => s.setViewport);
    const addObject = useBoardStore((s) => s.addObject);
    const selection = useBoardStore((s) => s.selection);
    const setSelection = useBoardStore((s) => s.setSelection);
    const clearSelection = useBoardStore((s) => s.clearSelection);
    const setTool = useBoardStore((s) => s.setTool);
    const presence = useBoardStore((s) => s.presence);

    const objectList = useMemo(() => Array.from(objects.values()), [objects]);
    const presenceList = useMemo(() => Array.from(presence.values()).filter(p => p.cursor), [presence]);

    // Track shape refs via callback
    const handleShapeRef = useCallback((id: string, node: Konva.Node | null) => {
        if (node) {
            nodeMapRef.current.set(id, node);
        } else {
            nodeMapRef.current.delete(id);
        }
    }, []);

    // Derive primitive key for selection
    const selectionKey = useMemo(
        () => Array.from(selection).sort().join(","),
        [selection],
    );

    // Attach transformer to selected nodes
    useEffect(() => {
        const tr = trRef.current;
        if (!tr) return;

        const ids = selectionKey ? selectionKey.split(",") : [];
        const nodes: Konva.Node[] = [];
        for (const id of ids) {
            const node = nodeMapRef.current.get(id);
            if (node) nodes.push(node);
        }
        tr.nodes(nodes);
        tr.getLayer()?.batchDraw();
    }, [selectionKey]);

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

    const handleStageMouseDown = useCallback(
        (e: KonvaEventObject<MouseEvent>) => {
            const target = e.target;
            // Don't deselect when clicking transformer anchors
            if (target.getParent()?.className === "Transformer") return;
            const clickedOnEmpty = target === target.getStage();
            if (clickedOnEmpty) {
                clearSelection();
            }
        },
        [clearSelection],
    );

    const handleMouseMove = useCallback(
        (e: KonvaEventObject<MouseEvent>) => {
            const now = Date.now();
            if (now - lastCursorSendRef.current < CURSOR_THROTTLE_MS) return;
            lastCursorSendRef.current = now;

            const stage = stageRef.current;
            if (!stage) return;

            const pointer = stage.getPointerPosition();
            if (!pointer) return;

            const canvasX = (pointer.x - viewport.x) / viewport.scale;
            const canvasY = (pointer.y - viewport.y) / viewport.scale;

            const store = useBoardStore.getState();
            const client = store.frameClient;
            const boardId = store.boardId;
            const user = store.user;
            if (!client || !boardId) return;

            client.send({
                id: crypto.randomUUID(),
                parent_id: null,
                ts: new Date().toISOString(),
                board_id: boardId,
                from: "client",
                syscall: "cursor:moved",
                status: "request",
                data: {
                    x: canvasX,
                    y: canvasY,
                    name: user?.name ?? "Anonymous",
                },
            });
        },
        [viewport],
    );

    const sendObjectCreate = useCallback((obj: BoardObject) => {
        const store = useBoardStore.getState();
        const client = store.frameClient;
        if (!client) return;

        const requestId = crypto.randomUUID();
        client.send({
            id: requestId,
            parent_id: null,
            ts: new Date().toISOString(),
            board_id: obj.board_id,
            from: "client",
            syscall: "object:create",
            status: "request",
            data: {
                kind: obj.kind,
                x: obj.x,
                y: obj.y,
                width: obj.width,
                height: obj.height,
                rotation: obj.rotation,
                z_index: obj.z_index,
                props: obj.props,
            },
        });

        // Listen for the response to replace temp ID with server ID
        const handleCreateResponse = (frame: Frame) => {
            if (frame.parent_id === requestId && frame.status === "item") {
                const serverId = frame.data.id as string;
                if (serverId && serverId !== obj.id) {
                    useBoardStore.getState().replaceObjectId(obj.id, serverId);
                }
                client.off("object:create", handleCreateResponse);
            }
        };
        client.on("object:create", handleCreateResponse);
    }, []);

    const handleStageClick = useCallback(
        (e: KonvaEventObject<MouseEvent>) => {
            // Only handle object creation on empty canvas click
            const clickedOnEmpty = e.target === e.target.getStage();
            if (!clickedOnEmpty) return;
            if (activeTool === "select") return;

            // Only handle tools that create objects
            const creatableTools = ["sticky", "rectangle", "ellipse"] as const;
            if (!creatableTools.includes(activeTool as typeof creatableTools[number])) return;

            const stage = stageRef.current;
            if (!stage) return;

            const pointer = stage.getPointerPosition();
            if (!pointer) return;

            const x = (pointer.x - viewport.x) / viewport.scale;
            const y = (pointer.y - viewport.y) / viewport.scale;

            let newObj: BoardObject;

            if (activeTool === "sticky") {
                const color =
                    NOTE_COLORS[Math.floor(Math.random() * NOTE_COLORS.length)];
                const rotation = (Math.random() - 0.5) * 4;
                newObj = {
                    id: crypto.randomUUID(),
                    board_id: useBoardStore.getState().boardId ?? "",
                    kind: "sticky_note",
                    x: x - 100,
                    y: y - 100,
                    width: 200,
                    height: 200,
                    rotation,
                    z_index: objects.size,
                    props: { color, text: "" },
                    created_by: "local",
                    version: 1,
                };
            } else {
                newObj = {
                    id: crypto.randomUUID(),
                    board_id: useBoardStore.getState().boardId ?? "",
                    kind: activeTool as "rectangle" | "ellipse",
                    x: x - 50,
                    y: y - 50,
                    width: 100,
                    height: 100,
                    rotation: 0,
                    z_index: objects.size,
                    props: {},
                    created_by: "local",
                    version: 1,
                };
            }

            addObject(newObj);
            sendObjectCreate(newObj);
            setSelection(new Set([newObj.id]));
            setTool("select");
        },
        [
            activeTool,
            viewport,
            objects.size,
            addObject,
            sendObjectCreate,
            setSelection,
            setTool,
        ],
    );

    return (
        <div style={{ background: "var(--canvas-bg)", width, height }}>
            <Stage
                ref={stageRef}
                width={width}
                height={height}
                scaleX={viewport.scale}
                scaleY={viewport.scale}
                x={viewport.x}
                y={viewport.y}
                onWheel={handleWheel}
                onMouseDown={handleStageMouseDown}
                onClick={handleStageClick}
                onMouseMove={handleMouseMove}
            >
                <Layer listening={false}>
                    <GridLines
                        width={width}
                        height={height}
                        viewport={viewport}
                        isDark={isDark}
                    />
                </Layer>
                <Layer>
                    {objectList.map((obj) => {
                        if (obj.kind === "sticky_note") {
                            return (
                                <StickyNote
                                    key={obj.id}
                                    object={obj}
                                    isSelected={selection.has(obj.id)}
                                    onSelect={() =>
                                        setSelection(new Set([obj.id]))
                                    }
                                    onShapeRef={handleShapeRef}
                                />
                            );
                        }
                        if (
                            obj.kind === "rectangle" ||
                            obj.kind === "ellipse"
                        ) {
                            return (
                                <Shape
                                    key={obj.id}
                                    object={obj}
                                    isSelected={selection.has(obj.id)}
                                    onSelect={() =>
                                        setSelection(new Set([obj.id]))
                                    }
                                    onShapeRef={handleShapeRef}
                                />
                            );
                        }
                        return null;
                    })}
                    {presenceList.map((p) => (
                        <RemoteCursor
                            key={p.user_id}
                            name={p.name}
                            color={p.color}
                            x={p.cursor!.x}
                            y={p.cursor!.y}
                            scale={viewport.scale}
                        />
                    ))}
                    <Transformer
                        ref={trRef}
                        flipEnabled={false}
                        rotateEnabled={true}
                        boundBoxFunc={(_oldBox, newBox) => {
                            if (Math.abs(newBox.width) < 5 || Math.abs(newBox.height) < 5) {
                                return _oldBox;
                            }
                            return newBox;
                        }}
                    />
                </Layer>
            </Stage>
        </div>
    );
}

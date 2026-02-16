import type Konva from "konva";
import type { KonvaEventObject } from "konva/lib/Node";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Layer, Line, Stage, Transformer } from "react-konva";
import { useCanvasSize } from "../hooks/useCanvasSize";
import type { BoardObject } from "../lib/types";
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

export function Canvas() {
    const { width, height } = useCanvasSize();
    const stageRef = useRef<Konva.Stage>(null);
    const transformerRef = useRef<Konva.Transformer>(null);
    const layerRef = useRef<Konva.Layer>(null);

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

    const objectList = useMemo(() => Array.from(objects.values()), [objects]);

    // Attach Transformer to selected nodes whenever selection or objects change.
    // We derive a primitive string key from the selection Set so the effect
    // dependency is a value comparison, not a reference comparison. This avoids
    // subtle issues where React/Zustand/useSyncExternalStore may skip re-renders
    // when comparing Set objects with Object.is in concurrent rendering scenarios.
    const selectionKey = useMemo(
        () => Array.from(selection).sort().join(","),
        [selection],
    );

    useEffect(() => {
        const tr = transformerRef.current;
        const layer = layerRef.current;
        if (!tr || !layer) return;

        // Parse IDs back from the primitive key
        const ids = selectionKey ? selectionKey.split(",") : [];
        const nodes: Konva.Node[] = [];
        for (const id of ids) {
            const node = layer.findOne(`.obj-${id}`);
            if (node) nodes.push(node);
        }
        tr.nodes(nodes);
        // Force synchronous redraw to ensure Transformer anchors render
        layer.draw();
    }, [selectionKey, objects]);

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

    const handleStageClick = useCallback(
        (e: KonvaEventObject<MouseEvent>) => {
            const target = e.target;
            // Ignore clicks on shapes, transformer anchors, etc.
            const isStage = target === target.getStage();
            const isTransformer = target.getParent()?.className === "Transformer";
            if (!isStage && !isTransformer) return;

            if (isTransformer) return; // Don't clear when clicking transformer controls

            if (activeTool === "select") {
                clearSelection();
                return;
            }

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
                    kind: activeTool,
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
            setSelection(new Set([newObj.id]));
            setTool("select");
        },
        [
            activeTool,
            viewport,
            objects.size,
            addObject,
            setSelection,
            clearSelection,
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
                onClick={handleStageClick}
            >
                <Layer listening={false}>
                    <GridLines
                        width={width}
                        height={height}
                        viewport={viewport}
                        isDark={isDark}
                    />
                </Layer>
                <Layer ref={layerRef}>
                    {objectList.map((obj) => {
                        if (obj.kind === "sticky_note") {
                            return <StickyNote key={obj.id} object={obj} />;
                        }
                        if (
                            obj.kind === "rectangle" ||
                            obj.kind === "ellipse"
                        ) {
                            return <Shape key={obj.id} object={obj} />;
                        }
                        return null;
                    })}
                    <Transformer
                        ref={transformerRef}
                        rotateEnabled={true}
                        padding={8}
                        anchorSize={8}
                        anchorCornerRadius={0}
                        borderStroke={isDark ? "#e8e0d2" : "#2c2824"}
                        borderStrokeWidth={1}
                        anchorStroke={isDark ? "#e8e0d2" : "#2c2824"}
                        anchorFill={isDark ? "#1c1a18" : "#f5f0e8"}
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

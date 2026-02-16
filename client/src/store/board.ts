import { create } from "zustand";
import type { BoardObject, Presence, ToolType, Viewport } from "../lib/types";

interface BoardState {
    boardId: string | null;
    objects: Map<string, BoardObject>;
    presence: Map<string, Presence>;
    selection: Set<string>;
    viewport: Viewport;
    activeTool: ToolType;

    setBoardId: (id: string | null) => void;
    setObjects: (objects: BoardObject[]) => void;
    addObject: (object: BoardObject) => void;
    updateObject: (id: string, partial: Partial<BoardObject>) => void;
    deleteObject: (id: string) => void;
    setPresence: (presence: Presence) => void;
    removePresence: (userId: string) => void;
    setSelection: (ids: Set<string>) => void;
    toggleSelection: (id: string) => void;
    clearSelection: () => void;
    setViewport: (viewport: Partial<Viewport>) => void;
    setTool: (tool: ToolType) => void;
}

export const useBoardStore = create<BoardState>((set) => ({
    boardId: null,
    objects: new Map(),
    presence: new Map(),
    selection: new Set(),
    viewport: { x: 0, y: 0, scale: 1 },
    activeTool: "select",

    setBoardId: (id) => set({ boardId: id }),

    setObjects: (objects) =>
        set({
            objects: new Map(objects.map((o) => [o.id, o])),
        }),

    addObject: (object) =>
        set((state) => {
            const next = new Map(state.objects);
            next.set(object.id, object);
            return { objects: next };
        }),

    updateObject: (id, partial) =>
        set((state) => {
            const existing = state.objects.get(id);
            if (!existing) return state;
            const next = new Map(state.objects);
            next.set(id, { ...existing, ...partial });
            return { objects: next };
        }),

    deleteObject: (id) =>
        set((state) => {
            const next = new Map(state.objects);
            next.delete(id);
            const selection = new Set(state.selection);
            selection.delete(id);
            return { objects: next, selection };
        }),

    setPresence: (presence) =>
        set((state) => {
            const next = new Map(state.presence);
            next.set(presence.user_id, presence);
            return { presence: next };
        }),

    removePresence: (userId) =>
        set((state) => {
            const next = new Map(state.presence);
            next.delete(userId);
            return { presence: next };
        }),

    setSelection: (ids) => set({ selection: ids }),

    toggleSelection: (id) =>
        set((state) => {
            const next = new Set(state.selection);
            if (next.has(id)) {
                next.delete(id);
            } else {
                next.add(id);
            }
            return { selection: next };
        }),

    clearSelection: () => set({ selection: new Set() }),

    setViewport: (partial) =>
        set((state) => ({
            viewport: { ...state.viewport, ...partial },
        })),

    setTool: (tool) => set({ activeTool: tool }),
}));

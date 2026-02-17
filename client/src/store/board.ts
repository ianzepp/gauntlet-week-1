import { create } from "zustand";
import type { FrameClient } from "../lib/frameClient";
import type {
    BoardObject,
    Presence,
    ToolType,
    User,
    Viewport,
} from "../lib/types";

type ConnectionStatus = "connecting" | "connected" | "disconnected";
export type RightTab = "inspector" | "ai";

export interface AiMessage {
    role: "user" | "assistant" | "error";
    text: string;
    mutations?: number;
}

interface BoardState {
    boardId: string | null;
    objects: Map<string, BoardObject>;
    presence: Map<string, Presence>;
    selection: Set<string>;
    viewport: Viewport;
    activeTool: ToolType;
    frameClient: FrameClient | null;
    connectionStatus: ConnectionStatus;
    user: User | null;
    aiMessages: AiMessage[];
    aiLoading: boolean;
    aiPanelOpen: boolean;
    activeRightTab: RightTab;
    rightPanelOpen: boolean;

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
    setFrameClient: (client: FrameClient | null) => void;
    setConnectionStatus: (status: ConnectionStatus) => void;
    setUser: (user: User | null) => void;
    replaceObjectId: (tempId: string, newId: string) => void;
    addAiMessage: (message: AiMessage) => void;
    setAiLoading: (loading: boolean) => void;
    toggleAiPanel: () => void;
    setRightTab: (tab: RightTab) => void;
    openRightPanel: (tab: RightTab) => void;
    closeRightPanel: () => void;
}

export const useBoardStore = create<BoardState>((set) => ({
    boardId: null,
    objects: new Map(),
    presence: new Map(),
    selection: new Set(),
    viewport: { x: 0, y: 0, scale: 1 },
    activeTool: "select",
    frameClient: null,
    connectionStatus: "disconnected",
    user: null,
    aiMessages: [],
    aiLoading: false,
    aiPanelOpen: false,
    activeRightTab: "inspector" as RightTab,
    rightPanelOpen: false,

    setBoardId: (id) => set({ boardId: id }),

    setObjects: (objects) =>
        set((state) => {
            const nextObjects = new Map(
                objects.map((o) => {
                    const existing = state.objects.get(o.id);
                    return [
                        o.id,
                        {
                            ...o,
                            localKey: existing?.localKey ?? o.localKey ?? o.id,
                        },
                    ];
                }),
            );
            const nextSelection = new Set(
                Array.from(state.selection).filter((id) => nextObjects.has(id)),
            );
            return { objects: nextObjects, selection: nextSelection };
        }),

    addObject: (object) =>
        set((state) => {
            const next = new Map(state.objects);
            next.set(object.id, {
                ...object,
                localKey: object.localKey ?? object.id,
            });
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

    setFrameClient: (client) => set({ frameClient: client }),
    setConnectionStatus: (status) => set({ connectionStatus: status }),
    setUser: (user) => set({ user }),

    replaceObjectId: (tempId, newId) =>
        set((state) => {
            const existing = state.objects.get(tempId);
            if (!existing) return state;
            const next = new Map(state.objects);
            next.delete(tempId);
            // Preserve the localKey so React key stays stable (no remount)
            next.set(newId, {
                ...existing,
                id: newId,
                localKey: existing.localKey ?? tempId,
            });
            const selection = new Set(state.selection);
            if (selection.has(tempId)) {
                selection.delete(tempId);
                selection.add(newId);
            }
            return { objects: next, selection };
        }),

    addAiMessage: (message) =>
        set((state) => ({ aiMessages: [...state.aiMessages, message] })),

    setAiLoading: (loading) => set({ aiLoading: loading }),

    toggleAiPanel: () =>
        set((state) => {
            if (state.rightPanelOpen && state.activeRightTab === "ai") {
                return { rightPanelOpen: false, aiPanelOpen: false };
            }
            return {
                rightPanelOpen: true,
                activeRightTab: "ai" as RightTab,
                aiPanelOpen: true,
            };
        }),

    setRightTab: (tab) =>
        set({ activeRightTab: tab, aiPanelOpen: tab === "ai" }),

    openRightPanel: (tab) =>
        set({
            rightPanelOpen: true,
            activeRightTab: tab,
            aiPanelOpen: tab === "ai",
        }),

    closeRightPanel: () => set({ rightPanelOpen: false, aiPanelOpen: false }),
}));

import { useCallback } from "react";
import type { Frame } from "../lib/types";
import { useBoardStore } from "../store/board";

export function useAI() {
    const sendPrompt = useCallback((text: string) => {
        const store = useBoardStore.getState();
        const { frameClient, boardId, connectionStatus } = store;

        if (!frameClient || connectionStatus !== "connected") {
            store.addAiMessage({
                role: "error",
                text: "Not connected to server",
            });
            return;
        }

        store.addAiMessage({ role: "user", text });
        store.setAiLoading(true);

        const requestId = crypto.randomUUID();
        console.log("[AI] sending prompt", { requestId, boardId, promptLen: text.length });

        const handler = (frame: Frame) => {
            if (frame.parent_id !== requestId) return;

            console.log("[AI] recv frame", { id: frame.id, status: frame.status, parentId: frame.parent_id });

            if (frame.status === "item") {
                const data = frame.data as { text?: string; mutations?: number };
                console.log("[AI] item received", { text: data.text?.slice(0, 80), mutations: data.mutations });
                useBoardStore.getState().addAiMessage({
                    role: "assistant",
                    text: data.text ?? "",
                    mutations: data.mutations,
                });
                useBoardStore.getState().setAiLoading(false);
                frameClient.off("ai:prompt", handler);
            } else if (frame.status === "error") {
                const data = frame.data as { message?: string };
                console.error("[AI] error received", data);
                useBoardStore.getState().addAiMessage({
                    role: "error",
                    text: data.message ?? "An error occurred",
                });
                useBoardStore.getState().setAiLoading(false);
                frameClient.off("ai:prompt", handler);
            } else if (frame.status === "done") {
                console.log("[AI] done received");
                // Ensure loading clears even if no item frame was received.
                useBoardStore.getState().setAiLoading(false);
                frameClient.off("ai:prompt", handler);
            }
        };

        frameClient.on("ai:prompt", handler);

        frameClient.send({
            id: requestId,
            parent_id: null,
            ts: Date.now(),
            board_id: boardId ?? null,
            from: null,
            syscall: "ai:prompt",
            status: "request",
            data: { prompt: text },
        });
    }, []);

    return { sendPrompt };
}

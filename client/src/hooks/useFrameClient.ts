import { useEffect, useRef } from "react";
import { FrameClient } from "../lib/frameClient";
import type { BoardObject, Frame } from "../lib/types";
import { useBoardStore } from "../store/board";

export function useFrameClient(
    mockMode = true,
): React.RefObject<FrameClient | null> {
    const clientRef = useRef<FrameClient | null>(null);

    useEffect(() => {
        const client = new FrameClient(mockMode);
        clientRef.current = client;

        const handleCreated = (frame: Frame) => {
            const obj = frame.data as unknown as BoardObject;
            if (obj?.id) {
                useBoardStore.getState().addObject(obj);
            }
        };

        const handleUpdated = (frame: Frame) => {
            const obj = frame.data as unknown as BoardObject;
            if (obj?.id) {
                useBoardStore.getState().updateObject(obj.id, obj);
            }
        };

        const handleDeleted = (frame: Frame) => {
            const id = frame.data.id as string;
            if (id) {
                useBoardStore.getState().deleteObject(id);
            }
        };

        client.on("object:created", handleCreated);
        client.on("object:updated", handleUpdated);
        client.on("object:deleted", handleDeleted);

        return () => {
            client.disconnect();
            clientRef.current = null;
        };
    }, [mockMode]);

    return clientRef;
}

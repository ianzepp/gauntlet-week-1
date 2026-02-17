import { useEffect, useRef } from "react";
import { createWsTicket } from "../lib/api";
import { FrameClient } from "../lib/frameClient";
import type { BoardObject, Frame, Presence } from "../lib/types";
import { useBoardStore } from "../store/board";

/** Maps request ID → temp object ID for locally-created objects */
const pendingCreates = new Map<string, string>();

export function sendObjectCreate(obj: BoardObject): void {
    const store = useBoardStore.getState();
    const client = store.frameClient;
    if (!client) return;

    const requestId = crypto.randomUUID();
    pendingCreates.set(requestId, obj.id);

    client.send({
        id: requestId,
        parent_id: null,
        ts: Date.now(),
        board_id: obj.board_id,
        from: null,
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
}

export function useFrameClient(
    mockMode = false,
): React.RefObject<FrameClient | null> {
    const clientRef = useRef<FrameClient | null>(null);

    useEffect(() => {
        const store = useBoardStore.getState();
        const client = new FrameClient(mockMode);
        clientRef.current = client;
        store.setFrameClient(client);
        store.setConnectionStatus("connecting");

        const handleSessionConnected = (frame: Frame) => {
            const s = useBoardStore.getState();
            s.setConnectionStatus("connected");

            // Send board:join if we have a boardId
            const boardId = s.boardId;
            if (boardId) {
                client.send({
                    id: crypto.randomUUID(),
                    parent_id: null,
                    ts: Date.now(),
                    board_id: boardId,
                    from: null,
                    syscall: "board:join",
                    status: "request",
                    data: {},
                });
            }
        };

        const handleBoardJoin = (frame: Frame) => {
            if (frame.status !== "done") return;

            if (frame.data.objects) {
                // Our join reply — load the object list
                const objects = frame.data.objects as unknown as BoardObject[];
                useBoardStore.getState().setObjects(objects);
            } else if (frame.data.client_id) {
                // Peer join broadcast — add presence with placeholder info
                // Name/color will be enriched on their first cursor:moved
                const clientId = frame.data.client_id as string;
                useBoardStore.getState().setPresence({
                    user_id: clientId,
                    name: "Joining…",
                    color: "#6366f1",
                    cursor: null,
                });
            }
        };

        const handleBoardPart = (frame: Frame) => {
            const clientId = frame.data.client_id as string;
            if (clientId) {
                useBoardStore.getState().removePresence(clientId);
            }
        };

        const handleObjectCreate = (frame: Frame) => {
            if (frame.status !== "done") return;

            const obj = frame.data as unknown as BoardObject;
            if (!obj?.id) return;

            const s = useBoardStore.getState();

            // If this client originated the request, reconcile temp ID
            if (frame.parent_id && pendingCreates.has(frame.parent_id)) {
                const tempId = pendingCreates.get(frame.parent_id)!;
                pendingCreates.delete(frame.parent_id);
                if (tempId !== obj.id) {
                    s.replaceObjectId(tempId, obj.id);
                }
                s.updateObject(obj.id, obj);
                return;
            }

            // Peer broadcast — add or update
            if (s.objects.has(obj.id)) {
                s.updateObject(obj.id, obj);
            } else {
                s.addObject(obj);
            }
        };

        const handleObjectUpdate = (frame: Frame) => {
            if (frame.status !== "done") return;
            const obj = frame.data as unknown as BoardObject;
            if (obj?.id) {
                useBoardStore.getState().updateObject(obj.id, obj);
            }
        };

        const handleObjectDelete = (frame: Frame) => {
            if (frame.status !== "done") return;
            const id = frame.data.id as string;
            if (id) {
                useBoardStore.getState().deleteObject(id);
            }
        };

        const handleCursorMoved = (frame: Frame) => {
            const data = frame.data;
            const clientId = data.client_id as string;
            if (!clientId) return;

            const presence: Presence = {
                user_id: clientId,
                name: (data.name as string) ?? "Anonymous",
                color: (data.color as string) ?? "#6366f1",
                cursor: { x: data.x as number, y: data.y as number },
            };
            useBoardStore.getState().setPresence(presence);
        };

        const handleChatMessage = (frame: Frame) => {
            const s = useBoardStore.getState();
            const message = (frame.data.message as string) ?? "";
            const fromId = (frame.data.from ?? frame.from ?? "") as string;

            // Resolve name/color from presence or current user
            let fromName = "Anonymous";
            let fromColor = "#6366f1";
            if (s.user && s.user.id === fromId) {
                fromName = s.user.name;
                fromColor = s.user.color;
            } else {
                const p = s.presence.get(fromId);
                if (p) {
                    fromName = p.name;
                    fromColor = p.color;
                }
            }

            s.addChatMessage({
                id: frame.id,
                ts: frame.ts,
                from: fromId,
                fromName,
                fromColor,
                message,
            });
        };

        const handleDisconnected = () => {
            useBoardStore.getState().setConnectionStatus("disconnected");
        };

        client.on("session:connected", handleSessionConnected);
        client.on("session:disconnected", handleDisconnected);
        client.on("board:join", handleBoardJoin);
        client.on("board:part", handleBoardPart);
        client.on("object:create", handleObjectCreate);
        client.on("object:update", handleObjectUpdate);
        client.on("object:delete", handleObjectDelete);
        client.on("cursor:moved", handleCursorMoved);
        client.on("chat:message", handleChatMessage);

        if (!mockMode) {
            const protocol =
                window.location.protocol === "https:" ? "wss:" : "ws:";
            const wsBase = `${protocol}//${window.location.host}`;

            createWsTicket().then((ticket) => {
                client.connect(`${wsBase}/api/ws`, ticket);
            });
        }

        return () => {
            client.disconnect();
            clientRef.current = null;
            const s = useBoardStore.getState();
            s.setFrameClient(null);
            s.setConnectionStatus("disconnected");
        };
    }, [mockMode]);

    return clientRef;
}

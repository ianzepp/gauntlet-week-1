import type { Frame } from "./types";

type FrameHandler = (frame: Frame) => void;

export class FrameClient {
    private mockMode: boolean;
    private ws: WebSocket | null = null;
    private handlers: Map<string, Set<FrameHandler>> = new Map();

    constructor(mockMode: boolean) {
        this.mockMode = mockMode;
    }

    connect(url: string, ticket: string): void {
        if (this.mockMode) {
            console.log("[FrameClient] mock mode — connected");
            this.dispatch({
                id: crypto.randomUUID(),
                parent_id: null,
                ts: Date.now(),
                board_id: "",
                from: "system",
                syscall: "session:connected",
                status: "done",
                data: {},
            });
            return;
        }

        const wsUrl = `${url}?ticket=${encodeURIComponent(ticket)}`;
        const ws = new WebSocket(wsUrl);
        this.ws = ws;

        ws.onmessage = (event) => {
            try {
                const frame = JSON.parse(event.data as string) as Frame;
                this.dispatch(frame);
            } catch {
                console.warn("[FrameClient] failed to parse frame:", event.data);
            }
        };

        ws.onclose = () => {
            this.ws = null;
            this.dispatch({
                id: crypto.randomUUID(),
                parent_id: null,
                ts: Date.now(),
                board_id: "",
                from: "system",
                syscall: "session:disconnected",
                status: "done",
                data: {},
            });
        };

        ws.onerror = (err) => {
            console.error("[FrameClient] ws error:", err);
        };
    }

    send(frame: Frame): void {
        if (this.mockMode) {
            console.log("[FrameClient] mock send:", frame.syscall, frame);
            return;
        }

        if (this.ws?.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(frame));
        }
    }

    on(syscall: string, handler: FrameHandler): void {
        if (!this.handlers.has(syscall)) {
            this.handlers.set(syscall, new Set());
        }
        this.handlers.get(syscall)?.add(handler);
    }

    off(syscall: string, handler: FrameHandler): void {
        this.handlers.get(syscall)?.delete(handler);
    }

    disconnect(): void {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        this.handlers.clear();
    }

    private dispatch(frame: Frame): void {
        const handlers = this.handlers.get(frame.syscall);
        if (handlers) {
            for (const handler of handlers) {
                handler(frame);
            }
        }
    }
}

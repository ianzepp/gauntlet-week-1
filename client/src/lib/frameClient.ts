import type { Frame } from "./types";

type FrameHandler = (frame: Frame) => void;

export class FrameClient {
    private mockMode: boolean;
    private handlers: Map<string, Set<FrameHandler>> = new Map();

    constructor(mockMode: boolean) {
        this.mockMode = mockMode;
    }

    connect(_url: string, _ticket: string): void {
        if (this.mockMode) {
            console.log("[FrameClient] mock mode — connected");
            this.dispatch({
                id: crypto.randomUUID(),
                parent_id: null,
                ts: new Date().toISOString(),
                board_id: "",
                from: "system",
                syscall: "session:connected",
                status: "done",
                data: {},
            });
            return;
        }
    }

    send(frame: Partial<Frame>): void {
        if (this.mockMode) {
            console.log("[FrameClient] mock send:", frame.syscall, frame);
            return;
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
        if (this.mockMode) {
            console.log("[FrameClient] mock mode — disconnected");
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

import { useEffect, useRef, useState } from "react";
import type { Frame } from "../lib/types";
import { useBoardStore } from "../store/board";
import styles from "./ChatPanel.module.css";

export function ChatPanel() {
    const messages = useBoardStore((s) => s.chatMessages);
    const frameClient = useBoardStore((s) => s.frameClient);
    const connectionStatus = useBoardStore((s) => s.connectionStatus);
    const boardId = useBoardStore((s) => s.boardId);
    const user = useBoardStore((s) => s.user);
    const setChatMessages = useBoardStore((s) => s.setChatMessages);

    const [input, setInput] = useState("");
    const messagesEndRef = useRef<HTMLDivElement>(null);
    const loadedRef = useRef(false);

    // Load chat history on mount
    useEffect(() => {
        if (!frameClient || connectionStatus !== "connected" || !boardId || loadedRef.current) return;
        loadedRef.current = true;

        const requestId = crypto.randomUUID();

        const handler = (frame: Frame) => {
            if (frame.parent_id !== requestId) return;
            if (frame.status === "done" && Array.isArray(frame.data.messages)) {
                const s = useBoardStore.getState();
                const history = (frame.data.messages as { id: string; ts: number; from: string; message: string }[]).map((m) => {
                    let fromName = "Anonymous";
                    let fromColor = "#6366f1";
                    if (s.user && s.user.id === m.from) {
                        fromName = s.user.name;
                        fromColor = s.user.color;
                    } else {
                        const p = s.presence.get(m.from);
                        if (p) {
                            fromName = p.name;
                            fromColor = p.color;
                        }
                    }
                    return {
                        id: m.id,
                        ts: m.ts,
                        from: m.from,
                        fromName,
                        fromColor,
                        message: m.message,
                    };
                });
                setChatMessages(history);
                frameClient.off("chat:history", handler);
            }
        };

        frameClient.on("chat:history", handler);
        frameClient.send({
            id: requestId,
            parent_id: null,
            ts: Date.now(),
            board_id: boardId,
            from: null,
            syscall: "chat:history",
            status: "request",
            data: {},
        });

        return () => {
            frameClient.off("chat:history", handler);
        };
    }, [frameClient, connectionStatus, boardId, setChatMessages]);

    // Auto-scroll on new messages
    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }, [messages]);

    const handleSubmit = () => {
        const text = input.trim();
        if (!text || !frameClient || connectionStatus !== "connected") return;
        setInput("");

        frameClient.send({
            id: crypto.randomUUID(),
            parent_id: null,
            ts: Date.now(),
            board_id: boardId,
            from: null,
            syscall: "chat:message",
            status: "request",
            data: { message: text },
        });
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            handleSubmit();
        }
    };

    return (
        <div className={styles.panel}>
            <div className={styles.messages}>
                {messages.map((msg) => (
                    <div key={msg.id} className={styles.message}>
                        <span
                            className={styles.name}
                            style={{ color: msg.fromColor }}
                        >
                            {msg.fromName}
                        </span>
                        <span className={styles.text}>{msg.message}</span>
                    </div>
                ))}
                <div ref={messagesEndRef} />
            </div>
            <div className={styles.inputArea}>
                <input
                    className={styles.input}
                    type="text"
                    placeholder={user?.name ? `Message as ${user.name}...` : "Type a message..."}
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={handleKeyDown}
                />
                <button
                    type="button"
                    className={styles.sendButton}
                    onClick={handleSubmit}
                    disabled={!input.trim()}
                >
                    Send
                </button>
            </div>
        </div>
    );
}

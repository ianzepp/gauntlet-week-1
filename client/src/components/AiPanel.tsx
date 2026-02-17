import { useEffect, useRef, useState } from "react";
import { useAI } from "../hooks/useAI";
import type { Frame } from "../lib/types";
import type { AiMessage } from "../store/board";
import { useBoardStore } from "../store/board";
import styles from "./AiPanel.module.css";

export function AiPanel() {
    const messages = useBoardStore((s) => s.aiMessages);
    const loading = useBoardStore((s) => s.aiLoading);
    const frameClient = useBoardStore((s) => s.frameClient);
    const connectionStatus = useBoardStore((s) => s.connectionStatus);
    const boardId = useBoardStore((s) => s.boardId);
    const { sendPrompt } = useAI();

    const [input, setInput] = useState("");
    const messagesEndRef = useRef<HTMLDivElement>(null);
    const loadedRef = useRef(false);

    // Load AI conversation history on mount
    useEffect(() => {
        if (!frameClient || connectionStatus !== "connected" || !boardId || loadedRef.current) return;
        if (messages.length > 0) {
            loadedRef.current = true;
            return;
        }
        loadedRef.current = true;

        const requestId = crypto.randomUUID();

        const handler = (frame: Frame) => {
            if (frame.parent_id !== requestId) return;
            if (frame.status === "done" && Array.isArray(frame.data.messages)) {
                const history = (frame.data.messages as { role: string; text: string; mutations?: number }[]).map((m) => ({
                    role: m.role as AiMessage["role"],
                    text: m.text,
                    mutations: m.mutations,
                }));
                // Prepend history before any current-session messages
                const current = useBoardStore.getState().aiMessages;
                const combined = [...history, ...current];
                // Replace all messages with combined
                useBoardStore.setState({ aiMessages: combined });
                frameClient.off("ai:history", handler);
            }
        };

        frameClient.on("ai:history", handler);
        frameClient.send({
            id: requestId,
            parent_id: null,
            ts: Date.now(),
            board_id: boardId,
            from: null,
            syscall: "ai:history",
            status: "request",
            data: {},
        });

        return () => {
            frameClient.off("ai:history", handler);
        };
    }, [frameClient, connectionStatus, boardId, messages.length]);

    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }, [messages, loading]);

    const handleSubmit = () => {
        const text = input.trim();
        if (!text || loading) return;
        setInput("");
        sendPrompt(text);
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
                {messages.map((msg, i) => (
                    <div
                        key={i}
                        className={`${styles.message} ${
                            msg.role === "user"
                                ? styles.messageUser
                                : msg.role === "error"
                                  ? styles.messageError
                                  : styles.messageAssistant
                        }`}
                    >
                        {msg.text}
                        {msg.mutations != null && msg.mutations > 0 && (
                            <div className={styles.mutations}>
                                {msg.mutations} object
                                {msg.mutations !== 1 ? "s" : ""} modified
                            </div>
                        )}
                    </div>
                ))}
                {loading && (
                    <div className={styles.loading}>Thinking...</div>
                )}
                <div ref={messagesEndRef} />
            </div>
            <div className={styles.inputArea}>
                <input
                    className={styles.input}
                    type="text"
                    placeholder="Ask AI to modify the board..."
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={handleKeyDown}
                    disabled={loading}
                />
                <button
                    type="button"
                    className={styles.sendButton}
                    onClick={handleSubmit}
                    disabled={loading || !input.trim()}
                >
                    Send
                </button>
            </div>
        </div>
    );
}

import { useEffect, useRef, useState } from "react";
import { useAI } from "../hooks/useAI";
import { useBoardStore } from "../store/board";
import styles from "./AiPanel.module.css";

export function AiPanel() {
    const messages = useBoardStore((s) => s.aiMessages);
    const loading = useBoardStore((s) => s.aiLoading);
    const { sendPrompt } = useAI();

    const [input, setInput] = useState("");
    const messagesEndRef = useRef<HTMLDivElement>(null);

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

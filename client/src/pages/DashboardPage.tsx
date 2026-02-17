import { useCallback, useEffect, useState } from "react";
import { BoardCard } from "../components/BoardCard";
import type { Frame } from "../lib/types";
import { useBoardStore } from "../store/board";
import styles from "./DashboardPage.module.css";

interface Board {
    id: string;
    name: string;
}

interface DashboardPageProps {
    onOpenBoard: (id: string, name: string) => void;
}

export function DashboardPage({ onOpenBoard }: DashboardPageProps) {
    const [boards, setBoards] = useState<Board[]>([]);
    const [showCreate, setShowCreate] = useState(false);
    const [newName, setNewName] = useState("");
    const frameClient = useBoardStore((s) => s.frameClient);
    const connectionStatus = useBoardStore((s) => s.connectionStatus);

    useEffect(() => {
        if (!frameClient || connectionStatus !== "connected") return;

        const requestId = crypto.randomUUID();

        const handler = (frame: Frame) => {
            if (frame.parent_id !== requestId) return;
            if (frame.status === "item") {
                const board = frame.data as unknown as Board;
                setBoards((prev) => [...prev, board]);
            }
        };

        frameClient.on("board:list", handler);

        frameClient.send({
            id: requestId,
            parent_id: null,
            ts: Date.now(),
            board_id: null,
            from: null,
            syscall: "board:list",
            status: "request",
            data: {},
        });

        return () => {
            frameClient.off("board:list", handler);
        };
    }, [frameClient, connectionStatus]);

    const handleCreate = useCallback(() => {
        if (!frameClient || !newName.trim()) return;

        const requestId = crypto.randomUUID();

        const handler = (frame: Frame) => {
            if (frame.parent_id !== requestId) return;
            if (frame.status === "done") {
                const boardId = frame.data.id as string;
                if (boardId) {
                    frameClient.off("board:create", handler);
                    onOpenBoard(boardId, newName.trim());
                }
            }
        };

        frameClient.on("board:create", handler);

        frameClient.send({
            id: requestId,
            parent_id: null,
            ts: Date.now(),
            board_id: null,
            from: null,
            syscall: "board:create",
            status: "request",
            data: { name: newName.trim() },
        });
    }, [frameClient, newName, onOpenBoard]);

    return (
        <div className={styles.page}>
            <div className={styles.header}>
                <span className={styles.headerTitle}>CollabBoard</span>
            </div>
            <div className={styles.grid}>
                <button
                    type="button"
                    className={styles.newCard}
                    onClick={() => {
                        setNewName("");
                        setShowCreate(true);
                    }}
                >
                    <svg viewBox="0 0 24 24" className={styles.newIcon}>
                        <path d="M12 5 L12 19 M5 12 L19 12" />
                    </svg>
                </button>
                {boards.map((board) => (
                    <BoardCard
                        key={board.id}
                        id={board.id}
                        name={board.name}
                        onClick={() => onOpenBoard(board.id, board.name)}
                    />
                ))}
            </div>

            {showCreate && (
                <div
                    className={styles.backdrop}
                    onClick={(e) => {
                        if (e.target === e.currentTarget) setShowCreate(false);
                    }}
                    onKeyDown={() => {}}
                    role="presentation"
                >
                    <div className={styles.dialog}>
                        <label className={styles.dialogLabel}>Board Name</label>
                        <input
                            className={styles.dialogInput}
                            value={newName}
                            onChange={(e) => setNewName(e.target.value)}
                            onKeyDown={(e) => {
                                if (e.key === "Enter") handleCreate();
                            }}
                            placeholder="Untitled Board"
                            autoFocus
                        />
                        <div className={styles.dialogActions}>
                            <button
                                type="button"
                                className={styles.dialogCancel}
                                onClick={() => setShowCreate(false)}
                            >
                                Cancel
                            </button>
                            <button
                                type="button"
                                className={styles.dialogButton}
                                onClick={handleCreate}
                            >
                                Create
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}

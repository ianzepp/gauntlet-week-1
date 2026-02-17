import { useEffect, useState } from "react";
import type { Frame } from "../lib/types";
import { useBoardStore } from "../store/board";
import { BoardCard } from "./BoardCard";
import styles from "./MissionControl.module.css";

interface Board {
    id: string;
    name: string;
}

interface MissionControlProps {
    currentBoardId: string;
    onSelectBoard: (id: string, name: string) => void;
}

export function MissionControl({
    currentBoardId,
    onSelectBoard,
}: MissionControlProps) {
    const [boards, setBoards] = useState<Board[]>([]);
    const frameClient = useBoardStore((s) => s.frameClient);
    const connectionStatus = useBoardStore((s) => s.connectionStatus);

    useEffect(() => {
        if (!frameClient || connectionStatus !== "connected") return;

        const requestId = crypto.randomUUID();

        const handler = (frame: Frame) => {
            if (frame.parent_id !== requestId) return;
            if (frame.status === "done" && Array.isArray(frame.data.boards)) {
                const list = frame.data.boards as unknown as Board[];
                setBoards(list);
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

    return (
        <div className={styles.bar}>
            <div className={styles.barInner}>
                {boards.map((board) => (
                    <BoardCard
                        key={board.id}
                        id={board.id}
                        name={board.name}
                        variant="mini"
                        active={board.id === currentBoardId}
                        onClick={() => onSelectBoard(board.id, board.name)}
                    />
                ))}
            </div>
        </div>
    );
}

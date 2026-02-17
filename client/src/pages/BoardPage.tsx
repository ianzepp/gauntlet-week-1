import { useEffect } from "react";
import { Canvas } from "../canvas/Canvas";
import { BoardStamp } from "../components/BoardStamp";
import { RightPanel } from "../components/RightPanel";
import { StatusBar } from "../components/StatusBar";
import { Toolbar } from "../components/Toolbar";
import { ToolRail } from "../components/ToolRail";
import { useBoardStore } from "../store/board";

interface BoardPageProps {
    boardId: string;
    boardName: string;
    onBack?: () => void;
}

export function BoardPage({ boardId, boardName, onBack }: BoardPageProps) {
    const setBoardId = useBoardStore((s) => s.setBoardId);
    const setBoardName = useBoardStore((s) => s.setBoardName);
    const clearPresence = useBoardStore((s) => s.clearPresence);
    const frameClient = useBoardStore((s) => s.frameClient);
    const connectionStatus = useBoardStore((s) => s.connectionStatus);

    useEffect(() => {
        setBoardId(boardId);
        setBoardName(boardName);
        return () => {
            setBoardId(null);
            setBoardName(null);
            clearPresence();
        };
    }, [boardId, boardName, setBoardId, setBoardName, clearPresence]);

    // Send board:join when connected and boardId is set
    useEffect(() => {
        if (!frameClient || connectionStatus !== "connected") return;

        frameClient.send({
            id: crypto.randomUUID(),
            parent_id: null,
            ts: Date.now(),
            board_id: boardId,
            from: null,
            syscall: "board:join",
            status: "request",
            data: {},
        });
    }, [frameClient, connectionStatus, boardId]);

    return (
        <div
            style={{
                display: "flex",
                flexDirection: "column",
                height: "100vh",
            }}
        >
            <Toolbar onBack={onBack} />
            <div style={{ flex: 1, overflow: "hidden", display: "flex" }}>
                <ToolRail />
                <div style={{ flex: 1, overflow: "hidden", position: "relative" }}>
                    <Canvas />
                    <BoardStamp />
                </div>
                <RightPanel />
            </div>
            <StatusBar />
        </div>
    );
}

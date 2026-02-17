import { useEffect } from "react";
import { Canvas } from "../canvas/Canvas";
import { BoardStamp } from "../components/BoardStamp";
import { LeftPanel } from "../components/LeftPanel";
import { RightPanel } from "../components/RightPanel";
import { StatusBar } from "../components/StatusBar";
import { Toolbar } from "../components/Toolbar";
import { useBoardStore } from "../store/board";

interface BoardPageProps {
    boardId: string;
    boardName: string;
    onBack?: () => void;
    onLogout?: () => void;
    onNavigate?: (id: string | null, name: string | null) => void;
}

export function BoardPage({ boardId, boardName, onBack, onLogout, onNavigate }: BoardPageProps) {
    const setBoardId = useBoardStore((s) => s.setBoardId);
    const setBoardName = useBoardStore((s) => s.setBoardName);
    const clearPresence = useBoardStore((s) => s.clearPresence);
    const frameClient = useBoardStore((s) => s.frameClient);
    const connectionStatus = useBoardStore((s) => s.connectionStatus);
    const setNavigateToBoard = useBoardStore((s) => s.setNavigateToBoard);

    useEffect(() => {
        setBoardId(boardId);
        setBoardName(boardName);
        // Clear per-board state so history reloads for the new board
        useBoardStore.setState({ aiMessages: [], chatMessages: [] });
        return () => {
            setBoardId(null);
            setBoardName(null);
            clearPresence();
        };
    }, [boardId, boardName, setBoardId, setBoardName, clearPresence]);

    // Expose navigate callback so MissionControl (in RightPanel) can trigger navigation
    useEffect(() => {
        if (onNavigate) {
            setNavigateToBoard((id: string, name: string) => onNavigate(id, name));
        }
        return () => setNavigateToBoard(null);
    }, [onNavigate, setNavigateToBoard]);

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
        <>
            <Canvas />
            <div
                style={{
                    position: "relative",
                    zIndex: 1,
                    display: "flex",
                    flexDirection: "column",
                    height: "100vh",
                    pointerEvents: "none",
                }}
            >
                <Toolbar onBack={onBack} onLogout={onLogout} />
                <div style={{ flex: 1, overflow: "hidden", display: "flex" }}>
                    <LeftPanel />
                    <div style={{ flex: 1, overflow: "hidden", position: "relative" }}>
                        <BoardStamp />
                    </div>
                    <RightPanel />
                </div>
                <StatusBar />
            </div>
        </>
    );
}

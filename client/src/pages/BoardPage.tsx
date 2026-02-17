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
}

export function BoardPage({ boardId }: BoardPageProps) {
    const setBoardId = useBoardStore((s) => s.setBoardId);

    useEffect(() => {
        setBoardId(boardId);
        return () => setBoardId(null);
    }, [boardId, setBoardId]);

    return (
        <div
            style={{
                display: "flex",
                flexDirection: "column",
                height: "100vh",
            }}
        >
            <Toolbar />
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

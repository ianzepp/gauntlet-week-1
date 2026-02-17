import { useEffect } from "react";
import { Canvas } from "../canvas/Canvas";
import { AiPanel } from "../components/AiPanel";
import { StatusBar } from "../components/StatusBar";
import { Toolbar } from "../components/Toolbar";
import { useBoardStore } from "../store/board";

interface BoardPageProps {
    boardId: string;
}

export function BoardPage({ boardId }: BoardPageProps) {
    const setBoardId = useBoardStore((s) => s.setBoardId);
    const aiPanelOpen = useBoardStore((s) => s.aiPanelOpen);

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
                <div style={{ flex: 1, overflow: "hidden" }}>
                    <Canvas />
                </div>
                {aiPanelOpen && <AiPanel />}
            </div>
            <StatusBar />
        </div>
    );
}

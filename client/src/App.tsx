import { useEffect, useState } from "react";
import { useFrameClient } from "./hooks/useFrameClient";
import { fetchCurrentUser } from "./lib/api";
import type { User } from "./lib/types";
import { BoardPage } from "./pages/BoardPage";
import { DashboardPage } from "./pages/DashboardPage";
import { LoginPage } from "./pages/LoginPage";
import { useBoardStore } from "./store/board";

function initDarkMode() {
    const saved = localStorage.getItem("gauntlet_week_1_dark");
    if (saved === "true") {
        document.documentElement.classList.add("dark-mode");
    } else if (saved === null) {
        const prefersDark = window.matchMedia(
            "(prefers-color-scheme: dark)",
        ).matches;
        if (prefersDark) {
            document.documentElement.classList.add("dark-mode");
            localStorage.setItem("gauntlet_week_1_dark", "true");
        }
    }
}

export function App() {
    const [user, setUser] = useState<User | null>(null);
    const [loading, setLoading] = useState(true);
    const [page, setPage] = useState<"dashboard" | "board">("dashboard");
    const [activeBoardId, setActiveBoardId] = useState<string | null>(null);
    const [activeBoardName, setActiveBoardName] = useState<string | null>(null);
    const setStoreUser = useBoardStore((s) => s.setUser);
    useFrameClient();

    useEffect(() => {
        initDarkMode();
    }, []);

    useEffect(() => {
        fetchCurrentUser()
            .then((u) => {
                setUser(u);
                setStoreUser(u);
            })
            .finally(() => setLoading(false));
    }, [setStoreUser]);

    if (loading) {
        return null;
    }

    if (!user) {
        return <LoginPage />;
    }

    if (page === "board" && activeBoardId) {
        return (
            <BoardPage
                boardId={activeBoardId}
                boardName={activeBoardName ?? "Untitled"}
                onBack={() => {
                    setPage("dashboard");
                    setActiveBoardId(null);
                    setActiveBoardName(null);
                }}
                onNavigate={(id, name) => {
                    if (id === null) {
                        setPage("dashboard");
                        setActiveBoardId(null);
                        setActiveBoardName(null);
                    } else {
                        setActiveBoardId(id);
                        setActiveBoardName(name);
                    }
                }}
            />
        );
    }

    return (
        <DashboardPage
            onOpenBoard={(id, name) => {
                setActiveBoardId(id);
                setActiveBoardName(name);
                setPage("board");
            }}
        />
    );
}

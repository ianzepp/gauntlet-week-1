import { useEffect, useState } from "react";
import { useFrameClient } from "./hooks/useFrameClient";
import { fetchCurrentUser } from "./lib/api";
import type { User } from "./lib/types";
import { BoardPage } from "./pages/BoardPage";
import { LoginPage } from "./pages/LoginPage";
import { useBoardStore } from "./store/board";

const DEMO_BOARD_ID = "demo-board-001";

function initDarkMode() {
    const saved = localStorage.getItem("collaboard_dark");
    if (saved === "true") {
        document.documentElement.classList.add("dark-mode");
    } else if (saved === null) {
        const prefersDark = window.matchMedia(
            "(prefers-color-scheme: dark)",
        ).matches;
        if (prefersDark) {
            document.documentElement.classList.add("dark-mode");
            localStorage.setItem("collaboard_dark", "true");
        }
    }
}

export function App() {
    const [user, setUser] = useState<User | null>(null);
    const [loading, setLoading] = useState(true);
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

    return <BoardPage boardId={DEMO_BOARD_ID} />;
}

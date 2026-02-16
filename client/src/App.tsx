import { useEffect, useState } from "react";
import { useFrameClient } from "./hooks/useFrameClient";
import type { User } from "./lib/types";
import { BoardPage } from "./pages/BoardPage";
import { LoginPage } from "./pages/LoginPage";

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

function loadUser(): User | null {
    const raw = localStorage.getItem("collaboard_user");
    if (!raw) return null;
    try {
        return JSON.parse(raw) as User;
    } catch {
        return null;
    }
}

export function App() {
    const [user, setUser] = useState<User | null>(loadUser);
    useFrameClient(true);

    useEffect(() => {
        initDarkMode();
    }, []);

    if (!user) {
        return <LoginPage onLogin={setUser} />;
    }

    return <BoardPage boardId={DEMO_BOARD_ID} />;
}

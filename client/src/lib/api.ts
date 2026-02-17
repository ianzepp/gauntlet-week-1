import type { User, UserProfile } from "./types";

export async function fetchCurrentUser(): Promise<User | null> {
    const resp = await fetch("/api/auth/me", { credentials: "include" });
    if (!resp.ok) return null;
    return resp.json();
}

export async function logout(): Promise<void> {
    await fetch("/api/auth/logout", {
        method: "POST",
        credentials: "include",
    });
}

export async function fetchUserProfile(userId: string): Promise<UserProfile | null> {
    const resp = await fetch(`/api/users/${userId}/profile`, {
        credentials: "include",
    });
    if (!resp.ok) return null;
    return resp.json();
}

export async function createWsTicket(): Promise<string> {
    const resp = await fetch("/api/auth/ws-ticket", {
        method: "POST",
        credentials: "include",
    });
    if (!resp.ok) throw new Error(`Failed to create WS ticket (${resp.status})`);
    const data = await resp.json();
    return data.ticket;
}

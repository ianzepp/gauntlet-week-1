export type FrameStatus = "request" | "item" | "done" | "error" | "cancel";

export interface Frame {
    id: string;
    parent_id: string | null;
    ts: number;
    board_id: string | null;
    from: string | null;
    syscall: string;
    status: FrameStatus;
    data: Record<string, unknown>;
}

export type ObjectKind =
    | "sticky_note"
    | "rectangle"
    | "ellipse"
    | "line"
    | "connector"
    | "text";

export interface BoardObject {
    id: string;
    board_id: string;
    kind: ObjectKind;
    x: number;
    y: number;
    width: number;
    height: number;
    rotation: number;
    z_index: number;
    props: Record<string, unknown>;
    created_by: string;
    version: number;
    /** Local-only stable key for React rendering; never sent to server */
    localKey?: string;
}

export interface Presence {
    user_id: string;
    name: string;
    color: string;
    cursor: { x: number; y: number } | null;
}

export interface Viewport {
    x: number;
    y: number;
    scale: number;
}

export type ToolType = "select" | "sticky" | "rectangle" | "ellipse" | "line" | "connector" | "text" | "draw" | "eraser";

export interface User {
    id: string;
    name: string;
    avatar_url?: string;
    color: string;
}

export interface UserProfile {
    id: string;
    name: string;
    avatar_url: string | null;
    color: string;
    member_since: string | null;
    stats: {
        total_frames: number;
        objects_created: number;
        boards_active: number;
        last_active: string | null;
        top_syscalls: { syscall: string; count: number }[];
    };
}

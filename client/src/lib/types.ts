export type FrameStatus = "request" | "item" | "done" | "error" | "cancel";

export interface Frame {
    id: string;
    parent_id: string | null;
    ts: string;
    board_id: string;
    from: string;
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

export type ToolType = "select" | "sticky" | "rectangle" | "ellipse";

export interface User {
    id: string;
    name: string;
}

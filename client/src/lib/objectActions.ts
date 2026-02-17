import type { BoardObject } from "./types";
import { useBoardStore } from "../store/board";

function collectExistingObjects(ids: string[]): BoardObject[] {
    const state = useBoardStore.getState();
    const seen = new Set<string>();
    const objects: BoardObject[] = [];
    for (const id of ids) {
        if (seen.has(id)) continue;
        seen.add(id);
        const obj = state.objects.get(id);
        if (obj) objects.push(obj);
    }
    return objects;
}

export function deleteObjectsWithConfirm(ids: string[]): boolean {
    const objects = collectExistingObjects(ids);
    if (objects.length === 0) return false;

    const message =
        objects.length === 1
            ? "Delete this object?"
            : `Delete ${objects.length} objects?`;
    if (!window.confirm(message)) return false;

    const state = useBoardStore.getState();
    for (const obj of objects) {
        state.deleteObject(obj.id);
        if (!state.frameClient) continue;
        state.frameClient.send({
            id: crypto.randomUUID(),
            parent_id: null,
            ts: Date.now(),
            board_id: obj.board_id,
            from: null,
            syscall: "object:delete",
            status: "request",
            data: { id: obj.id },
        });
    }

    return true;
}

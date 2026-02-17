import { useCallback, useEffect, useMemo, useState } from "react";
import type { BoardObject } from "../lib/types";
import { useBoardStore } from "../store/board";
import styles from "./InspectorPanel.module.css";

function normalizeHexColor(value: string | undefined, fallback: string): string {
    if (!value) return fallback;
    const trimmed = value.trim();
    const short = /^#([0-9a-fA-F]{3})$/;
    const full = /^#([0-9a-fA-F]{6})$/;

    const shortMatch = trimmed.match(short);
    if (shortMatch) {
        const s = shortMatch[1];
        return `#${s[0]}${s[0]}${s[1]}${s[1]}${s[2]}${s[2]}`.toLowerCase();
    }
    if (full.test(trimmed)) return trimmed.toLowerCase();
    return fallback;
}

function formatNumberInput(value: number | null): string {
    if (value == null || Number.isNaN(value)) return "";
    return String(Math.round(value));
}

function parseIntegerInput(value: string): number | null {
    const parsed = Number.parseInt(value, 10);
    if (Number.isNaN(parsed)) return null;
    return parsed;
}

export function InspectorPanel() {
    const selection = useBoardStore((s) => s.selection);
    const objects = useBoardStore((s) => s.objects);

    const selectedObjects = useMemo(
        () => Array.from(selection).map((id) => objects.get(id)).filter(Boolean) as BoardObject[],
        [objects, selection],
    );

    const obj = selectedObjects.length === 1 ? selectedObjects[0] : null;

    const [draftWidth, setDraftWidth] = useState("");
    const [draftHeight, setDraftHeight] = useState("");
    const [draftTitle, setDraftTitle] = useState("");
    const [draftText, setDraftText] = useState("");
    const [draftFontSize, setDraftFontSize] = useState("13");
    const [draftBackground, setDraftBackground] = useState("#d94b4b");
    const [draftBorder, setDraftBorder] = useState("#d94b4b");
    const [draftBorderWidth, setDraftBorderWidth] = useState("1");

    useEffect(() => {
        if (!obj) return;
        setDraftWidth(formatNumberInput(obj.width));
        setDraftHeight(formatNumberInput(obj.height));
        setDraftTitle((obj.props.title as string) ?? "");
        setDraftText((obj.props.text as string) ?? "");
        setDraftFontSize(formatNumberInput((obj.props.fontSize as number) ?? 13));

        const background = normalizeHexColor(
            (obj.props.backgroundColor as string) ?? (obj.props.color as string) ?? "#D94B4B",
            "#d94b4b",
        );
        const border = normalizeHexColor((obj.props.borderColor as string) ?? background, background);
        setDraftBackground(background);
        setDraftBorder(border);
        setDraftBorderWidth(formatNumberInput((obj.props.borderWidth as number) ?? 1));
    }, [obj]);

    const sendObjectUpdate = useCallback(
        (objectId: string, patch: Partial<BoardObject>) => {
            const state = useBoardStore.getState();
            const current = state.objects.get(objectId);
            if (!current) return;

            state.updateObject(objectId, patch);
            if (!state.frameClient) return;

            const data: Record<string, unknown> = {
                id: objectId,
                version: current.version,
            };

            if (patch.x != null) data.x = patch.x;
            if (patch.y != null) data.y = patch.y;
            if (patch.width != null) data.width = patch.width;
            if (patch.height != null) data.height = patch.height;
            if (patch.rotation != null) data.rotation = patch.rotation;
            if (patch.props != null) data.props = patch.props;

            state.frameClient.send({
                id: crypto.randomUUID(),
                parent_id: null,
                ts: Date.now(),
                board_id: current.board_id,
                from: null,
                syscall: "object:update",
                status: "request",
                data,
            });
        },
        [],
    );

    const commitDimension = (key: "width" | "height", value: string) => {
        if (!obj) return;
        const parsed = parseIntegerInput(value);
        const fallback = key === "width" ? obj.width ?? 0 : obj.height ?? 0;
        const next = Math.max(1, parsed ?? fallback);
        const current = key === "width" ? obj.width ?? 0 : obj.height ?? 0;
        if (Math.round(current) === Math.round(next)) {
            if (key === "width") setDraftWidth(formatNumberInput(current));
            else setDraftHeight(formatNumberInput(current));
            return;
        }
        if (key === "width") sendObjectUpdate(obj.id, { width: next });
        else sendObjectUpdate(obj.id, { height: next });
        if (key === "width") setDraftWidth(String(next));
        else setDraftHeight(String(next));
    };

    const commitText = () => {
        if (!obj) return;
        const current = (obj.props.text as string) ?? "";
        if (draftText === current) return;
        sendObjectUpdate(obj.id, { props: { ...obj.props, text: draftText } });
    };

    const commitTitle = () => {
        if (!obj) return;
        const current = (obj.props.title as string) ?? "";
        if (draftTitle === current) return;
        sendObjectUpdate(obj.id, { props: { ...obj.props, title: draftTitle } });
    };

    const commitFontSize = (value: string) => {
        if (!obj) return;
        const parsed = parseIntegerInput(value);
        const current = Math.max(1, Math.round((obj.props.fontSize as number) ?? 13));
        const next = Math.max(1, parsed ?? current);
        setDraftFontSize(String(next));
        if (next === current) return;
        sendObjectUpdate(obj.id, { props: { ...obj.props, fontSize: next } });
    };

    const commitBackground = (value: string) => {
        if (!obj) return;
        const next = normalizeHexColor(value, "#d94b4b");
        const current = normalizeHexColor(
            (obj.props.backgroundColor as string) ?? (obj.props.color as string) ?? "#d94b4b",
            "#d94b4b",
        );
        setDraftBackground(next);
        if (next === current) return;
        sendObjectUpdate(obj.id, {
            props: { ...obj.props, color: next, backgroundColor: next },
        });
    };

    const commitBorder = (value: string) => {
        if (!obj) return;
        const next = normalizeHexColor(value, draftBackground);
        const current = normalizeHexColor((obj.props.borderColor as string) ?? draftBackground, draftBackground);
        setDraftBorder(next);
        if (next === current) return;
        sendObjectUpdate(obj.id, { props: { ...obj.props, borderColor: next } });
    };

    const commitBorderWidth = (value: string) => {
        if (!obj) return;
        const parsed = parseIntegerInput(value);
        const current = Math.max(0, Math.round((obj.props.borderWidth as number) ?? 1));
        const next = Math.max(0, parsed ?? current);
        setDraftBorderWidth(String(next));
        if (next === current) return;
        sendObjectUpdate(obj.id, { props: { ...obj.props, borderWidth: next } });
    };

    if (selectedObjects.length === 0) {
        return (
            <div className={styles.panel}>
                <div className={styles.empty}>
                    <span className={styles.emptyLabel}>No selection</span>
                    <span className={styles.emptyHint}>Double click an object to inspect it.</span>
                </div>
            </div>
        );
    }

    if (!obj) {
        return (
            <div className={styles.panel}>
                <div className={styles.section}>
                    <span className={styles.objectKind}>{selectedObjects.length} objects selected</span>
                </div>
            </div>
        );
    }

    const kindLabel = obj.kind.replace("_", " ");

    return (
        <div className={styles.panel}>
            <div className={styles.section}>
                <span className={styles.objectKind}>{kindLabel}</span>
            </div>

            <div className={styles.section}>
                <span className={styles.sectionTitle}>Object Size</span>
                <div className={styles.sizeSummary}>
                    {Math.round(obj.width ?? 0)} x {Math.round(obj.height ?? 0)}
                </div>
                <div className={styles.fieldGrid}>
                    <label className={styles.fieldLabel} htmlFor="inspector-width">W</label>
                    <input
                        id="inspector-width"
                        className={styles.fieldInput}
                        inputMode="numeric"
                        value={draftWidth}
                        onChange={(e) => setDraftWidth(e.currentTarget.value)}
                        onBlur={() => commitDimension("width", draftWidth)}
                        onKeyDown={(e) => {
                            if (e.key === "Enter") {
                                e.preventDefault();
                                commitDimension("width", draftWidth);
                                e.currentTarget.blur();
                            }
                        }}
                    />
                    <label className={styles.fieldLabel} htmlFor="inspector-height">H</label>
                    <input
                        id="inspector-height"
                        className={styles.fieldInput}
                        inputMode="numeric"
                        value={draftHeight}
                        onChange={(e) => setDraftHeight(e.currentTarget.value)}
                        onBlur={() => commitDimension("height", draftHeight)}
                        onKeyDown={(e) => {
                            if (e.key === "Enter") {
                                e.preventDefault();
                                commitDimension("height", draftHeight);
                                e.currentTarget.blur();
                            }
                        }}
                    />
                </div>
            </div>

            <div className={styles.section}>
                <span className={styles.sectionTitle}>Text Content</span>
                {obj.kind === "sticky_note" && (
                    <div className={styles.inlineControl}>
                        <label className={styles.fieldLabel} htmlFor="inspector-title">Title</label>
                        <input
                            id="inspector-title"
                            className={styles.fieldInput}
                            value={draftTitle}
                            onChange={(e) => setDraftTitle(e.currentTarget.value)}
                            onBlur={commitTitle}
                            onKeyDown={(e) => {
                                if (e.key === "Enter") {
                                    e.preventDefault();
                                    commitTitle();
                                    e.currentTarget.blur();
                                }
                            }}
                        />
                    </div>
                )}
                <textarea
                    className={styles.textArea}
                    value={draftText}
                    onChange={(e) => setDraftText(e.currentTarget.value)}
                    onBlur={commitText}
                    placeholder="Type object text"
                />
                <div className={styles.inlineControl}>
                    <label className={styles.fieldLabel} htmlFor="inspector-font-size">Font Size</label>
                    <input
                        id="inspector-font-size"
                        className={styles.fieldInput}
                        inputMode="numeric"
                        value={draftFontSize}
                        onChange={(e) => setDraftFontSize(e.currentTarget.value)}
                        onBlur={() => commitFontSize(draftFontSize)}
                        onKeyDown={(e) => {
                            if (e.key === "Enter") {
                                e.preventDefault();
                                commitFontSize(draftFontSize);
                                e.currentTarget.blur();
                            }
                        }}
                    />
                </div>
            </div>

            <div className={styles.section}>
                <span className={styles.sectionTitle}>Appearance</span>
                <div className={styles.colorRow}>
                    <label className={styles.fieldLabel} htmlFor="inspector-background">Background</label>
                    <input
                        id="inspector-background"
                        className={styles.colorInput}
                        type="color"
                        value={draftBackground}
                        onChange={(e) => commitBackground(e.currentTarget.value)}
                    />
                </div>
                <div className={styles.colorRow}>
                    <label className={styles.fieldLabel} htmlFor="inspector-border">Border</label>
                    <input
                        id="inspector-border"
                        className={styles.colorInput}
                        type="color"
                        value={draftBorder}
                        onChange={(e) => commitBorder(e.currentTarget.value)}
                    />
                </div>
                <div className={styles.inlineControl}>
                    <label className={styles.fieldLabel} htmlFor="inspector-border-width">Border Width</label>
                    <input
                        id="inspector-border-width"
                        className={styles.fieldInput}
                        inputMode="numeric"
                        value={draftBorderWidth}
                        onChange={(e) => setDraftBorderWidth(e.currentTarget.value)}
                        onBlur={() => commitBorderWidth(draftBorderWidth)}
                        onKeyDown={(e) => {
                            if (e.key === "Enter") {
                                e.preventDefault();
                                commitBorderWidth(draftBorderWidth);
                                e.currentTarget.blur();
                            }
                        }}
                    />
                </div>
            </div>

            <div className={`${styles.section} ${styles.metaSection}`}>
                <span className={styles.sectionTitle}>Position / Meta</span>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>X</span>
                    <span className={styles.rowValue}>{Math.round(obj.x)}</span>
                </div>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>Y</span>
                    <span className={styles.rowValue}>{Math.round(obj.y)}</span>
                </div>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>Rot</span>
                    <span className={styles.rowValue}>{Math.round(obj.rotation)}°</span>
                </div>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>Z</span>
                    <span className={styles.rowValue}>{obj.z_index}</span>
                </div>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>Ver</span>
                    <span className={styles.rowValue}>{obj.version}</span>
                </div>
                <div className={styles.row}>
                    <span className={styles.rowLabel}>ID</span>
                    <span className={styles.rowValue}>{obj.id.slice(0, 8)}</span>
                </div>
            </div>
        </div>
    );
}

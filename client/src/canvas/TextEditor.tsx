import { useCallback, useEffect, useRef } from "react";

interface TextEditorProps {
    x: number;
    y: number;
    width: number;
    height: number;
    text: string;
    onSave: (text: string) => void;
    onCancel: () => void;
}

export function TextEditor({
    x,
    y,
    width,
    height,
    text,
    onSave,
    onCancel,
}: TextEditorProps) {
    const textareaRef = useRef<HTMLTextAreaElement>(null);

    useEffect(() => {
        const el = textareaRef.current;
        if (el) {
            el.focus();
            el.setSelectionRange(el.value.length, el.value.length);
        }
    }, []);

    const handleBlur = useCallback(() => {
        const value = textareaRef.current?.value ?? "";
        onSave(value);
    }, [onSave]);

    const handleKeyDown = useCallback(
        (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
            if (e.key === "Escape") {
                e.preventDefault();
                onCancel();
            } else if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                const value = textareaRef.current?.value ?? "";
                onSave(value);
            }
        },
        [onSave, onCancel],
    );

    return (
        <textarea
            ref={textareaRef}
            defaultValue={text}
            onBlur={handleBlur}
            onKeyDown={handleKeyDown}
            style={{
                position: "fixed",
                left: x,
                top: y + 44,
                width,
                height,
                fontFamily: "Caveat, cursive",
                fontSize: `${20 * (width / 200)}px`,
                padding: `${8 * (width / 200)}px`,
                background: "transparent",
                border: "none",
                outline: "none",
                resize: "none",
                color: "#2C2824",
                overflow: "hidden",
                zIndex: 1000,
            }}
        />
    );
}

import styles from "./BoardCard.module.css";

interface BoardCardProps {
    id: string;
    name: string;
    onClick: () => void;
    active?: boolean;
    variant?: "full" | "mini";
}

export function BoardCard({
    id,
    name,
    onClick,
    active = false,
    variant = "full",
}: BoardCardProps) {
    const className = [
        styles.card,
        variant === "mini" ? styles.mini : "",
        active ? styles.active : "",
    ]
        .filter(Boolean)
        .join(" ");

    return (
        <div
            className={className}
            onClick={onClick}
            onKeyDown={(e) => {
                if (e.key === "Enter") onClick();
            }}
            role="button"
            tabIndex={0}
        >
            <div className={styles.cardName}>{name}</div>
            <div className={styles.cardId}>{id.slice(0, 8)}</div>
            <div className={styles.cardPreview} />
        </div>
    );
}

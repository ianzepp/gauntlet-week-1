import { useEffect, useState } from "react";
import { fetchUserProfile } from "../lib/api";
import type { UserProfile } from "../lib/types";
import styles from "./UserFieldReport.module.css";

interface UserFieldReportProps {
    userId: string;
    anchorX: number;
    direction?: "up" | "down";
    onClose: () => void;
}

export function UserFieldReport({
    userId,
    anchorX,
    direction = "up",
    onClose,
}: UserFieldReportProps) {
    const [profile, setProfile] = useState<UserProfile | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        setLoading(true);
        fetchUserProfile(userId).then((p) => {
            setProfile(p);
            setLoading(false);
        });
    }, [userId]);

    // Position popover centered on the anchor, clamped to viewport
    const left = Math.max(8, Math.min(anchorX - 120, window.innerWidth - 248));
    const posStyle: React.CSSProperties =
        direction === "down"
            ? { left, top: 36, bottom: "auto" }
            : { left, bottom: 32, top: "auto" };

    return (
        <>
            <div className={styles.backdrop} onClick={onClose} />
            <div className={styles.popover} style={posStyle}>
                {loading ? (
                    <div className={styles.loading}>Loading field report...</div>
                ) : profile ? (
                    <>
                        <div className={styles.header}>
                            {profile.avatar_url && (
                                <img
                                    className={styles.avatar}
                                    src={profile.avatar_url}
                                    alt={profile.name}
                                />
                            )}
                            <div className={styles.headerInfo}>
                                <span className={styles.name}>
                                    {profile.name}
                                </span>
                                <span className={styles.badge}>
                                    Field Agent
                                    {profile.member_since
                                        ? ` // Since ${profile.member_since}`
                                        : ""}
                                </span>
                            </div>
                        </div>

                        <div className={styles.section}>
                            <div className={styles.sectionTitle}>
                                Activity Log
                            </div>
                            <div className={styles.row}>
                                <span className={styles.rowLabel}>
                                    Transmissions
                                </span>
                                <span className={styles.rowValue}>
                                    {profile.stats.total_frames}
                                </span>
                            </div>
                            <div className={styles.row}>
                                <span className={styles.rowLabel}>
                                    Objects Created
                                </span>
                                <span className={styles.rowValue}>
                                    {profile.stats.objects_created}
                                </span>
                            </div>
                            <div className={styles.row}>
                                <span className={styles.rowLabel}>
                                    Boards Active
                                </span>
                                <span className={styles.rowValue}>
                                    {profile.stats.boards_active}
                                </span>
                            </div>
                            {profile.stats.last_active && (
                                <div className={styles.row}>
                                    <span className={styles.rowLabel}>
                                        Last Signal
                                    </span>
                                    <span className={styles.rowValue}>
                                        {profile.stats.last_active}
                                    </span>
                                </div>
                            )}
                        </div>

                        {profile.stats.top_syscalls.length > 0 && (
                            <div className={styles.section}>
                                <div className={styles.sectionTitle}>
                                    Top Operations
                                </div>
                                {profile.stats.top_syscalls.map((s) => (
                                    <div
                                        key={s.syscall}
                                        className={styles.syscallBar}
                                    >
                                        <span className={styles.syscallName}>
                                            {s.syscall}
                                        </span>
                                        <span className={styles.syscallCount}>
                                            {s.count}
                                        </span>
                                    </div>
                                ))}
                            </div>
                        )}
                    </>
                ) : (
                    <div className={styles.loading}>Agent not found</div>
                )}
            </div>
        </>
    );
}

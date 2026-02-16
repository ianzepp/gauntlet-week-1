export function LoginPage() {
    const handleLogin = () => {
        window.location.href = "/auth/github";
    };

    return (
        <div
            style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                height: "100vh",
                background: "var(--bg-nav)",
            }}
        >
            <div
                style={{
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "center",
                    gap: "var(--space-lg)",
                }}
            >
                <h1
                    style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: "14px",
                        fontWeight: 700,
                        textTransform: "uppercase",
                        letterSpacing: "0.1em",
                        color: "var(--text-nav-active)",
                    }}
                >
                    CollabBoard
                </h1>
                <button
                    type="button"
                    onClick={handleLogin}
                    style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: "11px",
                        fontWeight: 600,
                        textTransform: "uppercase",
                        letterSpacing: "0.06em",
                        padding: "var(--space-sm) var(--space-md)",
                        background: "var(--accent-green)",
                        color: "var(--text-nav-active)",
                        border: "1px solid var(--accent-green)",
                        cursor: "pointer",
                    }}
                >
                    Login with GitHub
                </button>
            </div>
        </div>
    );
}

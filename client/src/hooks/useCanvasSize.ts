import { useEffect, useState } from "react";

const TOOLBAR_HEIGHT = 44;
const STATUSBAR_HEIGHT = 24;
const CHROME_HEIGHT = TOOLBAR_HEIGHT + STATUSBAR_HEIGHT;

export function useCanvasSize(): { width: number; height: number } {
    const [size, setSize] = useState({
        width: window.innerWidth,
        height: window.innerHeight - CHROME_HEIGHT,
    });

    useEffect(() => {
        const handleResize = () => {
            setSize({
                width: window.innerWidth,
                height: window.innerHeight - CHROME_HEIGHT,
            });
        };

        window.addEventListener("resize", handleResize);
        return () => window.removeEventListener("resize", handleResize);
    }, []);

    return size;
}

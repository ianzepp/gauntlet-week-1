import { useEffect, useState } from "react";

const TOOLBAR_HEIGHT = 44;

export function useCanvasSize(): { width: number; height: number } {
    const [size, setSize] = useState({
        width: window.innerWidth,
        height: window.innerHeight - TOOLBAR_HEIGHT,
    });

    useEffect(() => {
        const handleResize = () => {
            setSize({
                width: window.innerWidth,
                height: window.innerHeight - TOOLBAR_HEIGHT,
            });
        };

        window.addEventListener("resize", handleResize);
        return () => window.removeEventListener("resize", handleResize);
    }, []);

    return size;
}

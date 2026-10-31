/** Where a pointer is on a canvas, in canvas pixels (the canvas may be drawn at another size). */
export function canvasPoint(
    canvas: Pick<HTMLCanvasElement, "width" | "height" | "getBoundingClientRect">,
    clientX: number,
    clientY: number,
): { x: number; y: number } {
    const box = canvas.getBoundingClientRect();
    const scaleX = box.width === 0 ? 1 : canvas.width / box.width;
    const scaleY = box.height === 0 ? 1 : canvas.height / box.height;
    return { x: (clientX - box.left) * scaleX, y: (clientY - box.top) * scaleY };
}

/** Whether the canvas has to change size to match the size it is shown at. Never for an empty size. */
export function needsResize(canvas: Pick<HTMLCanvasElement, "width" | "height">, width: number, height: number): boolean {
    if (width === 0 || height === 0) return false;
    return canvas.width !== width || canvas.height !== height;
}

/** Gives the canvas a new size and keeps what was drawn on it, which resizing would erase. */
export function resizeKeepingDrawing(canvas: HTMLCanvasElement, width: number, height: number): void {
    const copy = document.createElement("canvas");
    copy.width = canvas.width;
    copy.height = canvas.height;
    copy.getContext("2d")?.drawImage(canvas, 0, 0);
    canvas.width = width;
    canvas.height = height;
    canvas.getContext("2d")?.drawImage(copy, 0, 0);
}

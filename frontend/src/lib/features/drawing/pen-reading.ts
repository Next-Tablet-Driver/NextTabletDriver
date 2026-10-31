/** What the test pad shows about the pointer that is over it. */
export interface PenReading {
    /** `pen` when Windows Ink is on, `mouse` when the pen acts as a mouse. */
    kind: string;
    /** From 0 to 1. */
    pressure: number;
    /** Degrees, from -90 to 90. */
    tiltX: number;
    tiltY: number;
    /** The eraser end of the pen is used. */
    eraser: boolean;
    /** The tip (or the eraser end) touches the pad. */
    down: boolean;
}

type PointerLike = Pick<PointerEvent, "pointerType" | "pressure" | "tiltX" | "tiltY" | "buttons">;

/** The `buttons` bit of a pen eraser. */
const ERASER_BUTTON = 32;
/** The `buttons` bit of the primary button, which is the pen tip. */
const PRIMARY_BUTTON = 1;

/** Reads the pen state out of a pointer event. */
export function readPointer(event: PointerLike): PenReading {
    const eraser = (event.buttons & ERASER_BUTTON) !== 0;
    return {
        kind: event.pointerType,
        pressure: event.pressure,
        tiltX: event.tiltX,
        tiltY: event.tiltY,
        eraser,
        down: eraser || (event.buttons & PRIMARY_BUTTON) !== 0,
    };
}

/** The line width for a pressure, so the stroke gets thicker as the pen presses harder. */
export function strokeWidth(pressure: number, min = 1, max = 14): number {
    const clamped = Math.min(1, Math.max(0, pressure));
    return min + (max - min) * clamped;
}

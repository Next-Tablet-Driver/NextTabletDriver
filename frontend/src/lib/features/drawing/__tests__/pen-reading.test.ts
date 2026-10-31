import { describe, it, expect } from "vitest";
import { readPointer, strokeWidth } from "../pen-reading";

const pen = { pointerType: "pen", pressure: 0.5, tiltX: 10, tiltY: -20, buttons: 1 };

describe("readPointer", () => {
    it("reads the type, pressure and tilt of a pen", () => {
        expect(readPointer(pen)).toEqual({ kind: "pen", pressure: 0.5, tiltX: 10, tiltY: -20, eraser: false, down: true });
    });

    it("tells a hovering pen from a touching one", () => {
        expect(readPointer({ ...pen, buttons: 0, pressure: 0 }).down).toBe(false);
    });

    it("detects the eraser end", () => {
        const reading = readPointer({ ...pen, buttons: 32 });
        expect(reading.eraser).toBe(true);
        expect(reading.down).toBe(true);
    });

    it("keeps the mouse type when Windows Ink is off", () => {
        expect(readPointer({ ...pen, pointerType: "mouse" }).kind).toBe("mouse");
    });
});

describe("strokeWidth", () => {
    it("is the thinnest line without pressure and the thickest at full pressure", () => {
        expect(strokeWidth(0, 1, 14)).toBe(1);
        expect(strokeWidth(1, 1, 14)).toBe(14);
    });

    it("grows with the pressure", () => {
        expect(strokeWidth(0.25)).toBeLessThan(strokeWidth(0.75));
    });

    it("stays in range for a pressure outside 0 to 1", () => {
        expect(strokeWidth(-1, 1, 14)).toBe(1);
        expect(strokeWidth(5, 1, 14)).toBe(14);
    });
});

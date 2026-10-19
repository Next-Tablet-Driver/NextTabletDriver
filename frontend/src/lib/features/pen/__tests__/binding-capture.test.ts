import { describe, it, expect } from "vitest";
import { buildChord, heldModifiers, keyName, modifierName } from "../binding-capture";

describe("keyName", () => {
    it.each([
        ["KeyA", "A"],
        ["KeyZ", "Z"],
        ["Digit0", "0"],
        ["Digit9", "9"],
        ["F1", "F1"],
        ["F12", "F12"],
        ["F24", "F24"],
        ["Space", "Space"],
        ["Escape", "Esc"],
        ["ArrowUp", "Up"],
        ["ArrowLeft", "Left"],
        ["PageDown", "PageDown"],
        ["Minus", "-"],
        ["Backquote", "`"],
        ["Backslash", "\\"],
        ["NumpadEnter", "Enter"],
    ])("names %s as %s", (code, expected) => {
        expect(keyName(code)).toBe(expected);
    });

    it.each(["F25", "F0", "KeyAB", "Digit10", "MediaPlayPause", "NumLock", "", "ShiftLeft"])(
        "has no name for %s",
        (code) => {
            expect(keyName(code)).toBeNull();
        },
    );
});

describe("modifierName", () => {
    it.each([
        ["ControlLeft", "Ctrl"],
        ["ControlRight", "Ctrl"],
        ["ShiftRight", "Shift"],
        ["AltLeft", "Alt"],
        ["MetaLeft", "Win"],
    ])("names %s as %s", (code, expected) => {
        expect(modifierName(code)).toBe(expected);
    });

    it("is null for a key that is not a modifier", () => {
        expect(modifierName("KeyA")).toBeNull();
    });
});

describe("buildChord", () => {
    it("writes the modifiers first, in a fixed order, then the key", () => {
        expect(buildChord(["Alt", "Shift", "Ctrl"], "Z")).toBe("Ctrl+Shift+Alt+Z");
    });

    it("does not repeat a modifier", () => {
        expect(buildChord(["Ctrl", "Ctrl"], "S")).toBe("Ctrl+S");
    });

    it("can be modifiers alone or a key alone", () => {
        expect(buildChord(["Shift"], null)).toBe("Shift");
        expect(buildChord([], "Space")).toBe("Space");
    });

    it("is empty when nothing was pressed", () => {
        expect(buildChord([], null)).toBe("");
    });
});

describe("heldModifiers", () => {
    it("lists the modifiers the event reports", () => {
        expect(heldModifiers({ ctrlKey: true, shiftKey: false, altKey: true, metaKey: false })).toEqual(["Ctrl", "Alt"]);
        expect(heldModifiers({ ctrlKey: false, shiftKey: false, altKey: false, metaKey: true })).toEqual(["Win"]);
        expect(heldModifiers({ ctrlKey: false, shiftKey: false, altKey: false, metaKey: false })).toEqual([]);
    });
});

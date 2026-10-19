/**
 * Turns keyboard events into the binding text the driver understands (`Ctrl+Shift+Z`).
 *
 * The names are the ones the driver prints back, and the driver is the only place that decides
 * whether a binding is valid: whatever is built here is still sent through `normalizeBinding`.
 * Keys are read from `event.code`, the position on the keyboard, because that is what the
 * driver presses on the other side whatever the keyboard layout is.
 */

/** Modifier keys, in the order they are written in a binding. */
const MODIFIER_NAMES: Readonly<Record<string, string>> = {
    ControlLeft: "Ctrl",
    ControlRight: "Ctrl",
    ShiftLeft: "Shift",
    ShiftRight: "Shift",
    AltLeft: "Alt",
    AltRight: "Alt",
    MetaLeft: "Win",
    MetaRight: "Win",
};

const MODIFIER_ORDER = ["Ctrl", "Shift", "Alt", "Win"] as const;

const NAMED_KEYS: Readonly<Record<string, string>> = {
    Escape: "Esc",
    Tab: "Tab",
    Enter: "Enter",
    NumpadEnter: "Enter",
    Space: "Space",
    Backspace: "Backspace",
    Delete: "Delete",
    Insert: "Insert",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Minus: "-",
    Equal: "=",
    BracketLeft: "[",
    BracketRight: "]",
    Backslash: "\\",
    Semicolon: ";",
    Quote: "'",
    Backquote: "`",
    Comma: ",",
    Period: ".",
    Slash: "/",
};

/** The most keys a binding can hold, as the driver enforces. */
export const MAX_CHORD_LENGTH = 4;

/** The modifier a key code stands for (`ControlLeft` is `Ctrl`), or `null` for any other key. */
export function modifierName(code: string): string | null {
    return MODIFIER_NAMES[code] ?? null;
}

/** The binding name of a non-modifier key code, or `null` when the driver cannot press it. */
export function keyName(code: string): string | null {
    const letter = /^Key([A-Z])$/.exec(code);
    if (letter?.[1] !== undefined) return letter[1];
    const digit = /^Digit([0-9])$/.exec(code);
    if (digit?.[1] !== undefined) return digit[1];
    if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
    return NAMED_KEYS[code] ?? null;
}

/** Joins modifiers and an optional key into binding text, modifiers first in a fixed order. */
export function buildChord(modifiers: Iterable<string>, key: string | null): string {
    const held = new Set(modifiers);
    const parts: string[] = MODIFIER_ORDER.filter((name) => held.has(name));
    if (key !== null) parts.push(key);
    return parts.join("+");
}

/** The binding names of the modifiers a keyboard event reports as held. */
export function heldModifiers(event: Pick<KeyboardEvent, "ctrlKey" | "shiftKey" | "altKey" | "metaKey">): string[] {
    const names: string[] = [];
    if (event.ctrlKey) names.push("Ctrl");
    if (event.shiftKey) names.push("Shift");
    if (event.altKey) names.push("Alt");
    if (event.metaKey) names.push("Win");
    return names;
}

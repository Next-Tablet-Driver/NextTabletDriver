/**
 * Development-only computed-style snapshots.
 *
 * Used to prove that a refactor of the CSS did not change how a screen looks: take a
 * snapshot before (`__ntdSnapshot.save("name")`), refactor, then compare
 * (`__ntdSnapshot.diff("name")`). Snapshots are stored by the Vite dev server in
 * `frontend/.snapshots/` (see `vite.config.ts`), which is git-ignored.
 *
 * Only imported behind `import.meta.env.DEV`.
 */

/** Computed properties compared for every element. */
const PROPS = [
    "display",
    "position",
    "color",
    "background-color",
    "background-image",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
    "border-top-style",
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-right-radius",
    "border-bottom-left-radius",
    "outline-color",
    "outline-width",
    "box-shadow",
    "opacity",
    "font-family",
    "font-size",
    "font-weight",
    "font-style",
    "line-height",
    "letter-spacing",
    "text-decoration-line",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "row-gap",
    "column-gap",
    "transition-duration",
    "cursor",
    "fill",
    "stroke",
] as const;

export type ElementStyles = Partial<Record<string, string>>;
export type Snapshot = Partial<Record<string, ElementStyles>>;

export interface StyleChange {
    path: string;
    prop: string;
    before: string;
    after: string;
}

export interface DiffResult {
    elementsBefore: number;
    elementsAfter: number;
    removed: string[];
    added: string[];
    changedCount: number;
    changed: StyleChange[];
}

function describe(el: Element): string {
    const id = el.id === "" ? "" : `#${el.id}`;
    const cls = [...el.classList]
        .filter((c) => !c.startsWith("svelte-"))
        .join(".");
    return `${el.tagName.toLowerCase()}${id}${cls === "" ? "" : `.${cls}`}`;
}

function pathOf(el: Element): string {
    const parts: string[] = [];
    let node: Element | null = el;
    while (node !== null && node !== document.documentElement) {
        const parent: Element | null = node.parentElement;
        const index = parent === null ? 0 : [...parent.children].indexOf(node);
        parts.unshift(`${describe(node)}[${String(index)}]`);
        node = parent;
    }
    return parts.join(" > ");
}

/**
 * `color-mix()` computes to `color(srgb r g b / a)`, whereas the hand-written rgba() it replaces
 * computes to `rgba(R, G, B, a)`. Convert the former so equal colors compare equal.
 */
function normalizeColors(value: string): string {
    return value.replace(
        /color\(srgb ([\d.]+) ([\d.]+) ([\d.]+)(?: \/ ([\d.]+))?\)/g,
        (_m, r: string, g: string, b: string, a: string | undefined) => {
            const channel = (v: string): string => String(Math.round(Number(v) * 255));
            const rgb = `${channel(r)}, ${channel(g)}, ${channel(b)}`;
            return a === undefined || Number(a) === 1 ? `rgb(${rgb})` : `rgba(${rgb}, ${String(Number(a))})`;
        },
    );
}

/** Snapshots the computed style of every element under `<body>` plus `<html>`. */
export function collect(): Snapshot {
    const snapshot: Snapshot = {};
    const elements = [document.documentElement, ...document.body.querySelectorAll("*")];
    for (const el of elements) {
        if (["SCRIPT", "STYLE", "LINK", "META", "TITLE", "HEAD"].includes(el.tagName)) continue;
        const computed = getComputedStyle(el);
        const styles: ElementStyles = {};
        for (const prop of PROPS) styles[prop] = normalizeColors(computed.getPropertyValue(prop));
        const rect = el.getBoundingClientRect();
        styles.box = `${String(Math.round(rect.width))}x${String(Math.round(rect.height))}`;
        snapshot[el === document.documentElement ? "html" : pathOf(el)] = styles;
    }
    return snapshot;
}

function validName(name: string): string {
    if (!/^[a-z0-9_-]{1,64}$/i.test(name)) throw new Error(`Invalid snapshot name: ${name}`);
    return name;
}

export function compare(before: Snapshot, after: Snapshot, limit = 60): DiffResult {
    const removed = Object.keys(before).filter((k) => !(k in after));
    const added = Object.keys(after).filter((k) => !(k in before));
    const changed: StyleChange[] = [];
    let changedCount = 0;
    for (const [path, styles] of Object.entries(before)) {
        const other = after[path];
        if (other === undefined || styles === undefined) continue;
        for (const [prop, value] of Object.entries(styles)) {
            const now = other[prop];
            if (now !== value) {
                changedCount++;
                if (changed.length < limit) changed.push({ path, prop, before: value ?? "", after: now ?? "" });
            }
        }
    }
    return {
        elementsBefore: Object.keys(before).length,
        elementsAfter: Object.keys(after).length,
        removed: removed.slice(0, limit),
        added: added.slice(0, limit),
        changedCount,
        changed,
    };
}

async function save(name: string): Promise<number> {
    const snapshot = collect();
    const response = await fetch(`/__snapshot/save?name=${validName(name)}`, {
        method: "POST",
        body: JSON.stringify(snapshot),
    });
    if (!response.ok) throw new Error(`Saving snapshot failed: ${String(response.status)}`);
    return Object.keys(snapshot).length;
}

async function diff(name: string, limit = 60): Promise<DiffResult> {
    const response = await fetch(`/__snapshot/load?name=${validName(name)}`);
    if (!response.ok) throw new Error(`No snapshot named ${name}`);
    const before = (await response.json()) as Snapshot;
    return compare(before, collect(), limit);
}

const PAGES = ["Output", "Filters", "Pen Settings", "Console", "Settings", "Release", "Credits"] as const;

/** Waits until the DOM stops changing (async pages render in several steps). */
async function settle(): Promise<void> {
    const wait = (ms: number): Promise<void> => new Promise((resolve) => { setTimeout(resolve, ms); });
    let last = "";
    let stable = 0;
    for (let i = 0; i < 40 && stable < 4; i++) {
        await wait(250);
        const state = `${String(document.body.querySelectorAll("*").length)}|${String(document.body.innerText.length)}`;
        if (state === last) stable++;
        else {
            stable = 0;
            last = state;
        }
    }
}

async function openPage(name: string): Promise<void> {
    const button = [...document.querySelectorAll("button")].find((b) => b.textContent.trim() === name);
    if (button === undefined) throw new Error(`No tab button named ${name}`);
    button.click();
    await settle();
}

/**
 * Hover styles depend on where the pointer happens to be, which would make two otherwise
 * identical runs differ. While snapshotting, nothing is hit-testable, so no `:hover` applies.
 */
function withoutPointer(): () => void {
    const style = document.createElement("style");
    style.textContent = "html.ntd-snapshot-mode, html.ntd-snapshot-mode * { pointer-events: none !important; }";
    document.head.append(style);
    document.documentElement.classList.add("ntd-snapshot-mode");
    return () => {
        document.documentElement.classList.remove("ntd-snapshot-mode");
        style.remove();
    };
}

/** The theme currently applied (`dark`, `light`, `catppuccinmocha`...). */
function currentTheme(): string {
    const cls = [...document.documentElement.classList].find((c) => c.startsWith("theme-"));
    return cls === undefined ? "dark" : cls.replace("theme-", "");
}

function keyFor(prefix: string, page: string): string {
    return `${prefix}-${currentTheme()}-${page.replace(" ", "")}`;
}

/**
 * Saves one snapshot per page for the current theme: `<prefix>-<theme>-<page>`.
 * Run against `/?mock&theme=<Theme>`: the theme comes from the (mocked) config, because the app
 * re-applies the configured theme and would override a class set by hand.
 */
async function saveAllPages(prefix: string): Promise<Record<string, number>> {
    const restore = withoutPointer();
    const saved: Record<string, number> = {};
    for (const page of PAGES) {
        await openPage(page);
        const key = keyFor(prefix, page);
        saved[key] = await save(key);
    }
    restore();
    return saved;
}

/** Compares every page of the current theme against `saveAllPages(prefix)`. */
async function diffAllPages(prefix: string, limit = 8): Promise<Record<string, DiffResult | "identical">> {
    const restore = withoutPointer();
    const results: Record<string, DiffResult | "identical"> = {};
    for (const page of PAGES) {
        await openPage(page);
        const key = keyFor(prefix, page);
        const result = await diff(key, limit);
        const differs = result.changedCount + result.added.length + result.removed.length > 0;
        results[key] = differs ? result : "identical";
    }
    restore();
    return results;
}

declare global {
    interface Window {
        __ntdSnapshot?: {
            save: typeof save;
            diff: typeof diff;
            collect: typeof collect;
            saveAllPages: typeof saveAllPages;
            diffAllPages: typeof diffAllPages;
        };
    }
}

export function installStyleSnapshot(): void {
    window.__ntdSnapshot = { save, diff, collect, saveAllPages, diffAllPages };
}

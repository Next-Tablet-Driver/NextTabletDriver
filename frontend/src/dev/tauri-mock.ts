/**
 * Development-only Tauri mock.
 *
 * Lets the real app run in a plain browser (`npm run dev`, then open `/?mock`) with fixture
 * data, so every screen can be rendered, screenshotted and compared without the native
 * window. It is only ever imported behind `import.meta.env.DEV`, so it is absent from
 * production builds.
 */
import type { Config, LogEntry, PluginManifest, UntrustedPlugin } from "../lib/infrastructure/tauri/commands";

type Handler = (args: Record<string, unknown>) => unknown;

const config: Config = {
    target_area: { x: 0, y: 0, w: 1920, h: 1080 },
    active_area: { x: 112, y: 74, w: 224, h: 148, rotation: 0 },
    display_snapping: true,
    lock_aspect_ratio: true,
    show_osu_playfield: true,
    mode: "Absolute",
    plugins: {
        kalman_smoothing: { enabled: true, properties: { process_noise: 0.5, latency: 4 } },
    },
    tip_threshold: 10,
    eraser_threshold: 10,
    disable_pressure: false,
    disable_tilt: false,
    tip_binding: "Left Click",
    eraser_binding: "Disabled",
    pen_button_bindings: ["Right Click", "Middle Click"],
    run_at_startup: false,
    system_tray_on_minimize: true,
    force_high_resolution_timer: true,
    websocket: {
        enabled: true,
        port: 8765,
        polling_rate_hz: 60,
        send_coordinates: true,
        send_pressure: true,
        send_tilt: false,
        send_status: true,
    },
    pressure_curve: {
        curve_type: "Custom",
        exponent: 2,
        points: [
            [0, 0],
            [0.25, 0.12],
            [0.5, 0.4],
            [0.75, 0.72],
            [1, 1],
        ],
    },
    theme: new URLSearchParams(window.location.search).get("theme") ?? "Dark",
    language: "English",
};

const LEVELS = ["INFO", "DEBUG", "WARN", "ERROR"] as const;
const GROUPS = ["HID", "TabletManager", "Config", "Plugins", "WebSocket", "PerfSpike"] as const;

function makeLogs(): LogEntry[] {
    const logs: LogEntry[] = [];
    for (let i = 0; i < 28; i++) {
        const level = LEVELS[i % LEVELS.length] ?? "INFO";
        const group = GROUPS[i % GROUPS.length] ?? "HID";
        const message = `Sample ${level.toLowerCase()} message number ${String(i + 1)} from ${group}`;
        logs.push({
            time: `12:${String(10 + Math.floor(i / 6)).padStart(2, "0")}:${String((i * 7) % 60).padStart(2, "0")}`,
            level,
            group,
            message,
            search_text: `${group.toLowerCase()} ${message.toLowerCase()}`,
        });
    }
    return logs;
}

const plugins: PluginManifest[] = [
    {
        id: "kalman_smoothing",
        name: "Kalman Smoothing",
        version: "1.0.0",
        author: "NextTablet",
        description: "Smooths pen movement with a Kalman filter.",
        icon: null,
        properties: [
            {
                id: "process_noise",
                name: "Process noise",
                tooltip: "Higher values follow the pen more closely.",
                kind: { type: "Float", config: { min: 0, max: 5, step: 0.1, default: 0.5, unit: "" } },
            },
            {
                id: "latency",
                name: "Latency",
                tooltip: null,
                kind: { type: "Int", config: { min: 0, max: 20, step: 1, default: 4, unit: "ms" } },
            },
            {
                id: "invert",
                name: "Invert",
                tooltip: null,
                kind: { type: "Bool", config: { default: false } },
            },
            {
                id: "mode",
                name: "Mode",
                tooltip: null,
                kind: { type: "Choice", config: { options: ["Fast", "Balanced", "Smooth"], default_index: 1 } },
            },
        ],
    },
    {
        id: "hand_speed",
        name: "Hand Speed",
        version: "1.2.0",
        author: "nzbasic",
        description: "Tracks hand speed and travel distance.",
        icon: null,
        properties: [],
    },
];

const untrusted: UntrustedPlugin[] = [{ file_name: "mystery_filter.dll", sha256: "a".repeat(64) }];

const releases = [
    {
        tag_name: "v2.0.0",
        name: "Release v2.0.0",
        published_at: "2026-10-01T12:00:00Z",
        assets: [],
        body: [
            "## What's Changed",
            "* New Tauri interface by @iSweat-exe in https://github.com/Next-Tablet-Driver/NextTabletDriver/pull/101",
            "* Fix pressure curve editor by @contributor-one in https://github.com/Next-Tablet-Driver/NextTabletDriver/pull/102",
            "* Improve plugin loading by @contributor-two in https://github.com/Next-Tablet-Driver/NextTabletDriver/pull/103",
            "**Full Changelog**: https://github.com/Next-Tablet-Driver/NextTabletDriver/compare/v1.9.0...v2.0.0",
        ].join("\n"),
    },
    {
        tag_name: "v1.9.0",
        name: "Release v1.9.0",
        published_at: "2026-08-12T09:30:00Z",
        assets: [],
        body: "* Add WebSocket overlay support by @iSweat-exe in https://github.com/Next-Tablet-Driver/NextTabletDriver/pull/90",
    },
];

const contributors = [
    { login: "iSweat-exe", avatar_url: "", html_url: "https://github.com/iSweat-exe", contributions: 412 },
    { login: "contributor-one", avatar_url: "", html_url: "https://github.com/contributor-one", contributions: 37 },
    { login: "contributor-two", avatar_url: "", html_url: "https://github.com/contributor-two", contributions: 12 },
];

const handlers = new Map<string, Handler>([
    [
        "list_themes",
        () => [
            {
                id: "neon-night",
                file_name: "neon-night.json",
                content: JSON.stringify({
                    schema: 1,
                    metadata: { name: "Neon Night", author: "Demo", version: "1.0" },
                    base: "dark",
                    tokens: { accent: "#ff2bd6", "accent-hover": "#ff6be6", "bg-app": "#0d0b1a", "bg-panel": "#16122b", "radius-md": "8px" },
                }),
            },
            { id: "broken", file_name: "broken.json", content: JSON.stringify({ schema: 1, metadata: { name: "Broken" }, tokens: { accent: "red" } }) },
        ],
    ],
    ["import_theme", () => null],
    ["delete_theme", () => null],
    ["get_core_version", () => "v2.0.0"],
    ["get_tablet_status", () => ({
        connected: true,
        name: "Wacom Intuos Pro M",
        width: 224,
        height: 148,
        max_x: 44800,
        max_y: 29600,
    })],
    ["get_config", () => config],
    ["set_config", () => null],
    ["save_config", () => null],
    ["reset_config", () => null],
    ["get_metrics", () => ({ hz: 998, last_packet_count: 123456 })],
    ["get_presets", () => [
        { name: "osu!", path: "osu.json" },
        { name: "Drawing", path: "drawing.json" },
    ]],
    ["get_current_profile_name", () => "Default Profile"],
    ["get_logs", () => makeLogs()],
    ["clear_logs", () => null],
    ["get_releases", () => releases],
    ["get_available_plugins", () => plugins],
    ["reload_plugins", () => plugins],
    ["get_untrusted_plugins", () => untrusted],
    ["open_plugins_folder", () => null],
    ["open_themes_folder", () => null],
    ["install_plugin", () => false],
    ["trust_plugin", () => false],
    ["delete_plugin", () => null],
    ["load_profile", () => null],
    ["import_otd_profile", () => null],
    ["export_profile", () => null],
]);

/** Fixtures for the GitHub REST calls the Credits page makes directly with `fetch`. */
function mockGithubFetch(): void {
    const realFetch = window.fetch.bind(window);
    window.fetch = (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
        const url = input instanceof Request ? input.url : String(input);
        const json = (body: unknown): Promise<Response> =>
            Promise.resolve(new Response(JSON.stringify(body), { status: 200, headers: new Headers([["content-type", "application/json"]]) }));
        if (url.includes("/contributors")) return json(contributors);
        if (url.includes("/releases")) {
            return json([{ assets: [{ name: "setup.exe", download_count: 1520 }, { name: "latest.json", download_count: 99 }] }]);
        }
        if (url.startsWith("https://api.github.com/repos/")) return json({ stargazers_count: 321 });
        return realFetch(input, init);
    };
}

/** Installs a minimal `__TAURI_INTERNALS__` so the Tauri JS APIs work against fixtures. */
export function installTauriMock(): void {
    const callbacks = new Map<number, (payload: unknown) => void>();
    let nextCallbackId = 1;

    window.__TAURI_INTERNALS__ = {
        metadata: {
            currentWindow: { label: "main" },
            currentWebview: { windowLabel: "main", label: "main" },
        },
        transformCallback: (callback: (payload: unknown) => void): number => {
            const id = nextCallbackId++;
            callbacks.set(id, callback);
            return id;
        },
        unregisterCallback: (id: number): void => {
            callbacks.delete(id);
        },
        convertFileSrc: (path: string): string => path,
        invoke: (command: string, args: Record<string, unknown> = {}): Promise<unknown> => {
            const handler = handlers.get(command);
            if (handler !== undefined) return Promise.resolve(handler(args));
            if (command === "plugin:event|listen") return Promise.resolve(nextCallbackId++);
            if (command === "plugin:window|is_maximized") return Promise.resolve(false);
            // Anything else (window controls, updater, dialogs...) is a harmless no-op.
            return Promise.resolve(null);
        },
    };

    mockGithubFetch();
}

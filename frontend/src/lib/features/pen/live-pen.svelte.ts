import type { UnlistenFn } from "@tauri-apps/api/event";
import { events, type TabletEventData } from "../../infrastructure/events";

/** The tablet statuses that mean the pen is near enough to be followed on the curve. */
const PEN_NEAR = new Set(["Hover", "Contact", "Active", "Eraser", "Pen"]);

/**
 * What the pen is doing right now, for the pressure curve to draw it.
 *
 * `ratio` is the raw pressure as a share of the tablet's maximum (0 to 1), before the curve.
 * It is fed by the same tablet events as the rest of the app, so it only moves while the window
 * is visible; call `start` when the view appears and `stop` when it goes away.
 */
export class LivePen {
    /** Raw pressure, from 0 to 1. */
    ratio = $state(0);
    /** The pen is near the tablet. */
    present = $state(false);
    /** The tip touches the tablet. */
    touching = $state(false);

    #unlisten: UnlistenFn | undefined;
    /** Counts the starts and stops, so a start that is answered after a stop can tell. */
    #run = 0;

    async start(): Promise<void> {
        const run = ++this.#run;
        const unlisten = await events.onTabletEvent((data) => { this.update(data); });
        if (run !== this.#run) {
            unlisten?.();
            return;
        }
        this.#unlisten = unlisten;
    }

    stop(): void {
        this.#run++;
        this.#unlisten?.();
        this.#unlisten = undefined;
        this.present = false;
        this.touching = false;
        this.ratio = 0;
    }

    /** Applies one tablet event. */
    update(data: TabletEventData): void {
        const max = data.max_pressure ?? 0;
        this.present = PEN_NEAR.has(data.status);
        this.touching = data.status === "Contact" || data.status === "Active";
        this.ratio = max > 0 ? Math.min(1, Math.max(0, data.pressure / max)) : 0;
    }
}

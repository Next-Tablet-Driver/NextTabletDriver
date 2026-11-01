import { useEffect, useState } from "react";
import { log } from "@/infrastructure/log";
import { getMetrics } from "@/infrastructure/tauri/commands";
import { DetachedFrame } from "@/shell/DetachedFrame";
import { Page, Section } from "@/ui/Page";

const REFRESH_MS = 500;
const HISTORY = 60;
const WIDTH = 300;
const HEIGHT = 80;

/** A polyline of `values`, scaled to the largest one. */
export function sparkline(values: readonly number[], width: number, height: number): string {
  const max = Math.max(1, ...values);
  const step = values.length > 1 ? width / (values.length - 1) : 0;
  return values
    .map((v, i) => `${(i * step).toFixed(1)},${(height - (v / max) * height).toFixed(1)}`)
    .join(" ");
}

/** How fast packets arrive, now and over the last half minute. */
export function PerformancePage() {
  const [rates, setRates] = useState<number[]>([]);
  const [packets, setPackets] = useState(0);

  useEffect(() => {
    const refresh = (): void => {
      getMetrics().then(
        (metrics) => {
          setRates((previous) => [...previous, metrics.hz ?? 0].slice(-HISTORY));
          setPackets(metrics.last_packet_count);
        },
        (e: unknown) => {
          log.error("Failed to read the metrics:", e);
        },
      );
    };
    refresh();
    const timer = setInterval(refresh, REFRESH_MS);
    return () => {
      clearInterval(timer);
    };
  }, []);

  const current = rates.at(-1) ?? 0;

  return (
    <DetachedFrame title="NextTabletDriver - Input Lag Analysis">
      <Page
        title="Input lag analysis"
        description="The rate at which the tablet reports, which sets how smooth the cursor feels."
      >
        <Section title="Report rate">
          <div className="flex flex-wrap items-end gap-8">
            <div className="flex flex-col">
              <span className="text-display-md font-semibold text-primary tabular-nums">
                {Math.round(current)} Hz
              </span>
              <span className="text-sm text-tertiary">{packets} packets in the last interval</span>
            </div>
            <svg
              role="img"
              aria-label={`Report rate over the last ${String(HISTORY / 2)} seconds`}
              viewBox={`0 0 ${String(WIDTH)} ${String(HEIGHT)}`}
              className="h-20 w-full max-w-sm"
            >
              <polyline
                fill="none"
                points={sparkline(rates, WIDTH, HEIGHT)}
                className="stroke-fg-brand-primary"
                strokeWidth="2"
                strokeLinejoin="round"
              />
            </svg>
          </div>
        </Section>
      </Page>
    </DetachedFrame>
  );
}

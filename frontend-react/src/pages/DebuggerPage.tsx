import { useState } from "react";
import { Badge } from "@/components/base/badges/badges";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { events, type TabletEventData } from "@/infrastructure/tauri/events";
import { DetachedFrame } from "@/shell/DetachedFrame";
import { Page, Section } from "@/ui/Page";

const onTabletEvent = (callback: (data: TabletEventData) => void) => events.onTabletEvent(callback);

const FIELDS: { label: string; read: (packet: TabletEventData) => string }[] = [
  { label: "X", read: (p) => String(p.x) },
  { label: "Y", read: (p) => String(p.y) },
  { label: "Pressure", read: (p) => String(p.pressure) },
  { label: "Hover distance", read: (p) => String(p.hover_distance) },
  { label: "Tilt X", read: (p) => String(p.tilt_x) },
  { label: "Tilt Y", read: (p) => String(p.tilt_y) },
];

/** The latest packet of the pen, as the driver parsed it. */
export function DebuggerPage() {
  const [packet, setPacket] = useState<TabletEventData | null>(null);
  useTauriEvent(onTabletEvent, setPacket);

  return (
    <DetachedFrame title="NextTabletDriver - Debugger">
      <Page title="Tablet debugger" description="The latest packet the driver read from the pen.">
        <Section
          title="Packet"
          actions={
            <Badge type="pill-color" size="md" color={packet === null ? "gray" : "success"}>
              {packet === null ? "Waiting for the pen" : packet.status}
            </Badge>
          }
        >
          <dl className="grid grid-cols-2 gap-4 md:grid-cols-3">
            {FIELDS.map(({ label, read }) => (
              <div key={label} className="flex flex-col gap-1">
                <dt className="text-sm text-tertiary">{label}</dt>
                <dd className="font-mono text-lg font-semibold text-primary tabular-nums">
                  {packet === null ? "–" : read(packet)}
                </dd>
              </div>
            ))}
          </dl>
        </Section>
      </Page>
    </DetachedFrame>
  );
}

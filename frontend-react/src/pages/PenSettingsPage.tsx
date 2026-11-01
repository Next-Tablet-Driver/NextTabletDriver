import { useEffect, useState } from "react";
import { Toggle } from "@/components/base/toggle/toggle";
import { InputNumber } from "@/components/base/input/input-number";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { BindingField } from "@/features/pen/BindingField";
import { PressureCurveEditor } from "@/features/pen/PressureCurveEditor";
import type { Config } from "@/infrastructure/tauri/commands";
import { getTabletStatus } from "@/infrastructure/tauri/commands";
import { Page, Row, Section } from "@/ui/Page";

type Kind = "pen" | "aux";

/** How many buttons the connected tablet says it has; 0 while unknown. */
function useButtonCounts(): { pen: number; aux: number } {
  const [counts, setCounts] = useState({ pen: 0, aux: 0 });
  useEffect(() => {
    getTabletStatus().then(
      (status) => {
        setCounts({ pen: status.pen_button_count, aux: status.aux_button_count });
      },
      // Outside the app (tests, a plain browser) there is no tablet to ask.
      () => undefined,
    );
  }, []);
  return counts;
}

/** The thresholds, the bindings of the pen and of the tablet keys, and the pressure curve. */
export function PenSettingsPage() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  const counts = useButtonCounts();

  if (config === null) {
    return null;
  }

  const set = (change: Partial<Config>): void => {
    mapping.update((c) => ({ ...c, ...change }));
  };

  const listOf = (kind: Kind): string[] =>
    kind === "pen" ? config.pen_button_bindings : config.aux_button_bindings;
  /** A row for every binding already saved, and for every button the tablet really has. */
  const rows = (kind: Kind): number => Math.max(listOf(kind).length, counts[kind]);
  /** Writes one binding, padding the list with `None` up to its position. */
  const edit = (kind: Kind, index: number, text: string): void => {
    const list = [...listOf(kind)];
    while (list.length <= index) {
      list.push("None");
    }
    list[index] = text;
    set(kind === "pen" ? { pen_button_bindings: list } : { aux_button_bindings: list });
  };

  return (
    <Page title="Pen Settings" description="How the pen tip, the eraser and the buttons behave.">
      <Section title="Pressure">
        <Row label="Tip threshold" hint="The raw pressure at which the tip counts as touching.">
          <div className="w-28">
            <InputNumber
              aria-label="Tip"
              size="sm"
              minValue={0}
              maxValue={1024}
              formatOptions={{ useGrouping: false }}
              value={config.tip_threshold}
              onChange={(tip_threshold) => {
                if (Number.isFinite(tip_threshold)) {
                  set({ tip_threshold });
                }
              }}
            />
          </div>
        </Row>
        <Row label="Eraser threshold" hint="The same, for the eraser end of the pen.">
          <div className="w-28">
            <InputNumber
              aria-label="Eraser"
              size="sm"
              minValue={0}
              maxValue={1024}
              formatOptions={{ useGrouping: false }}
              value={config.eraser_threshold}
              onChange={(eraser_threshold) => {
                if (Number.isFinite(eraser_threshold)) {
                  set({ eraser_threshold });
                }
              }}
            />
          </div>
        </Row>
        <Row label="Ignore pressure" hint="Every touch is sent at full pressure.">
          <Toggle
            aria-label="Disable pressure"
            isSelected={config.disable_pressure}
            onChange={(disable_pressure) => {
              set({ disable_pressure });
            }}
          />
        </Row>
        <Row label="Ignore tilt" hint="The angle of the pen is not sent.">
          <Toggle
            aria-label="Disable tilt"
            isSelected={config.disable_tilt}
            onChange={(disable_tilt) => {
              set({ disable_tilt });
            }}
          />
        </Row>
      </Section>

      <PressureCurveEditor />

      <Section title="Tip and eraser" description="What touching the tablet does.">
        <BindingField
          label="Tip binding"
          value={config.tip_binding}
          onEdit={(tip_binding) => {
            set({ tip_binding });
          }}
        />
        <BindingField
          label="Eraser binding"
          value={config.eraser_binding}
          onEdit={(eraser_binding) => {
            set({ eraser_binding });
          }}
        />
      </Section>

      <Section title="Pen buttons" description="What each button on the side of the pen does.">
        {rows("pen") === 0 ? (
          <p className="py-2 text-sm text-tertiary">
            No pen button is known yet. Connect a tablet to set them.
          </p>
        ) : (
          Array.from({ length: rows("pen") }, (_, index) => (
            <BindingField
              key={index}
              label={`Pen button ${String(index + 1)}`}
              value={config.pen_button_bindings[index] ?? "None"}
              onEdit={(text) => {
                edit("pen", index, text);
              }}
            />
          ))
        )}
      </Section>

      {rows("aux") > 0 && (
        <Section title="Tablet keys" description="What each key on the tablet itself does.">
          {Array.from({ length: rows("aux") }, (_, index) => (
            <BindingField
              key={index}
              label={`Tablet key ${String(index + 1)}`}
              value={config.aux_button_bindings[index] ?? "None"}
              onEdit={(text) => {
                edit("aux", index, text);
              }}
            />
          ))}
        </Section>
      )}
    </Page>
  );
}

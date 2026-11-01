import { useEffect, useState } from "react";
import { BindingField } from "@/features/pen/BindingField";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { getTabletStatus } from "@/infrastructure/tauri/commands";
import styles from "@/features/pen/ButtonActions.module.css";

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

const PEN_ICON = (
  <svg
    fill="none"
    height="18"
    stroke="currentColor"
    strokeLinecap="round"
    strokeLinejoin="round"
    strokeWidth="2"
    viewBox="0 0 24 24"
    width="18"
  >
    <rect height="20" rx="4" width="12" x="6" y="2" />
    <line x1="12" x2="12.01" y1="18" y2="18" />
  </svg>
);

const KEYS_ICON = (
  <svg
    fill="none"
    height="18"
    stroke="currentColor"
    strokeLinecap="round"
    strokeLinejoin="round"
    strokeWidth="2"
    viewBox="0 0 24 24"
    width="18"
  >
    <rect height="16" rx="2" width="20" x="2" y="4" />
    <path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M8 12h.01M12 12h.01M16 12h.01M7 16h10" />
  </svg>
);

/** The pen buttons and the tablet keys, one binding field each. */
export function ButtonActions() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  const counts = useButtonCounts();

  if (config === null) {
    return null;
  }

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
    mapping.update((c) =>
      kind === "pen" ? { ...c, pen_button_bindings: list } : { ...c, aux_button_bindings: list },
    );
  };

  return (
    <>
      <div className={`${styles.card ?? ""} ${styles["buttons-card"] ?? ""}`}>
        <div className={`${styles["section-header"] ?? ""} ${styles.small ?? ""}`}>
          {PEN_ICON}
          <h3>Button Actions</h3>
        </div>
        {Array.from({ length: rows("pen") }, (_, index) => (
          <BindingField
            key={index}
            id={`pen_button_${String(index)}`}
            label={`Pen Button ${String(index + 1)}`}
            value={config.pen_button_bindings[index] ?? "None"}
            onEdit={(text) => {
              edit("pen", index, text);
            }}
          />
        ))}
      </div>
      {rows("aux") > 0 && (
        <div className={`${styles.card ?? ""} ${styles["buttons-card"] ?? ""}`}>
          <div className={`${styles["section-header"] ?? ""} ${styles.small ?? ""}`}>
            {KEYS_ICON}
            <h3>Tablet Keys</h3>
          </div>
          {Array.from({ length: rows("aux") }, (_, index) => (
            <BindingField
              key={index}
              id={`aux_button_${String(index)}`}
              label={`Tablet Key ${String(index + 1)}`}
              value={config.aux_button_bindings[index] ?? "None"}
              onEdit={(text) => {
                edit("aux", index, text);
              }}
            />
          ))}
        </div>
      )}
    </>
  );
}

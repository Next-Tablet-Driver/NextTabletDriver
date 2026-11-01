import { Pencil } from "lucide-react";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { Button } from "@/ui/Button";
import styles from "@/features/pen/BindingCard.module.css";

/** The binding of the pen tip or of the eraser. */
export function BindingCard({ type }: { type: "tip" | "eraser" }) {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  const value =
    config === null ? null : type === "tip" ? config.tip_binding : config.eraser_binding;

  return (
    <div className={styles.card}>
      <div className={styles["card-title"]}>
        {type === "tip" ? "Tip Binding" : "Eraser Binding"}
      </div>
      <div className={styles["binding-input"]}>
        {value !== null && (
          <input
            type="text"
            value={value}
            onChange={(event) => {
              const text = event.currentTarget.value;
              mapping.update((c) =>
                type === "tip" ? { ...c, tip_binding: text } : { ...c, eraser_binding: text },
              );
            }}
          />
        )}
        <Button aria-label={`Edit ${type} Binding`} variant="segment">
          <Pencil size={14} />
        </Button>
      </div>
    </div>
  );
}

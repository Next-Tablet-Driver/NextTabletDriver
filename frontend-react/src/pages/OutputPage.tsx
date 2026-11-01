import { useEffect } from "react";
import { DisplayVisualizer } from "@/features/mapping/DisplayVisualizer";
import { useMappingStore } from "@/features/mapping/MappingProvider";
import { TabletVisualizer } from "@/features/mapping/TabletVisualizer";
import styles from "@/pages/OutputPage.module.css";

/** Ctrl+Z undoes, Ctrl+Shift+Z goes back to the loaded profile, Ctrl+Y redoes, Ctrl+S saves. */
function useEditShortcuts(): void {
  const mapping = useMappingStore();
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent): void => {
      if (!e.ctrlKey) {
        return;
      }
      const key = e.key.toLowerCase();
      if (key === "z") {
        e.preventDefault();
        if (e.shiftKey) {
          mapping.revert();
        } else {
          mapping.undo();
        }
      } else if (key === "y" && !e.shiftKey) {
        e.preventDefault();
        mapping.redo();
      } else if (key === "s" && !e.shiftKey) {
        e.preventDefault();
        void mapping.save();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [mapping]);
}

/** Where on the screens the pen draws, and which part of the tablet it uses. */
export function OutputPage() {
  useEditShortcuts();
  return (
    <div className={styles.page}>
      <DisplayVisualizer />
      <TabletVisualizer />
    </div>
  );
}

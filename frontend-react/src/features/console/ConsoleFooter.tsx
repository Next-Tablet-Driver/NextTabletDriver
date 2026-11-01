import { Copy, Download, Trash } from "lucide-react";
import { Button } from "@/ui/Button";
import styles from "@/features/console/ConsoleFooter.module.css";

export function ConsoleFooter({
  onClear,
  onCopy,
  onExport,
  filteredCount,
  totalCount,
}: {
  onClear: () => void;
  onCopy: () => void;
  onExport: () => void;
  filteredCount: number;
  totalCount: number;
}) {
  return (
    <div className={styles["console-footer"]}>
      <div className={styles["footer-actions"]}>
        <Button onClick={onClear}>
          <Trash size={14} /> Clear Console
        </Button>
        <Button onClick={onCopy}>
          <Copy size={14} /> Copy Visible
        </Button>
        <Button onClick={onExport}>
          <Download size={14} /> Export All
        </Button>
      </div>
      <div className={styles["footer-stats"]}>
        Showing {filteredCount} / {totalCount} logs
      </div>
    </div>
  );
}

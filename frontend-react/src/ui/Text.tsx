import type { ReactNode } from "react";
import { classes } from "@/ui/classes";
import styles from "@/ui/Text.module.css";

export function Text({ muted = false, children }: { muted?: boolean; children: ReactNode }) {
  return <p className={classes(styles.text, muted && styles.muted)}>{children}</p>;
}

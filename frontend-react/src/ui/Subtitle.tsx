import type { ReactNode } from "react";
import styles from "@/ui/Subtitle.module.css";

export function Subtitle({ children }: { children: ReactNode }) {
  return <h3 className={styles.subtitle}>{children}</h3>;
}

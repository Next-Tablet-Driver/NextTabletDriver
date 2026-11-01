import type { ReactNode } from "react";
import styles from "@/ui/Title.module.css";

export function Title({ children }: { children: ReactNode }) {
  return <h2 className={styles.title}>{children}</h2>;
}

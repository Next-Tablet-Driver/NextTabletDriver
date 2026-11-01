import type { LucideIcon } from "lucide-react";
import styles from "@/features/credits/StatCard.module.css";

export function StatCard({
  icon: Icon,
  value,
  name,
}: {
  icon: LucideIcon;
  value: string;
  name: string;
}) {
  return (
    <div className={styles["stat-card"]}>
      <Icon className={styles["stat-icon"]} size={22} />
      <div className={styles["stat-info"]}>
        <span className={styles["stat-value"]}>{value}</span>
        <span className={styles["stat-name"]}>{name}</span>
      </div>
    </div>
  );
}

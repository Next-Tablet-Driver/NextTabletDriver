import { classes } from "@/ui/classes";
import styles from "@/ui/Tabs.module.css";

export function Tabs({
  activeTab,
  tabs,
  onSelect,
}: {
  activeTab: string;
  tabs: string[];
  onSelect: (tab: string) => void;
}) {
  return (
    <div className={styles.tabs}>
      {tabs.map((tab) => (
        <button
          key={tab}
          className={classes(styles.tab, activeTab === tab && styles.active)}
          onClick={() => {
            onSelect(tab);
          }}
        >
          {tab}
        </button>
      ))}
    </div>
  );
}

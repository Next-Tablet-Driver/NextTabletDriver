import { Search, X } from "lucide-react";
import { LEVELS, type Level } from "@/features/console/logs";
import { classes } from "@/ui/classes";
import styles from "@/features/console/ConsoleToolbar.module.css";

export function ConsoleToolbar({
  searchQuery,
  onSearch,
  shown,
  onToggle,
}: {
  searchQuery: string;
  onSearch: (query: string) => void;
  shown: ReadonlySet<Level>;
  onToggle: (level: Level) => void;
}) {
  return (
    <div className={styles["console-toolbar"]}>
      <div className={styles["search-box"]}>
        <Search className={styles["search-icon"]} size={14} />
        <input
          placeholder="Search logs..."
          type="text"
          value={searchQuery}
          onChange={(event) => {
            onSearch(event.currentTarget.value);
          }}
        />
        {searchQuery.length > 0 && (
          <button
            className={styles["clear-search"]}
            aria-label="Clear search"
            onClick={() => {
              onSearch("");
            }}
          >
            <X size={14} />
          </button>
        )}
      </div>
      <div className={styles["filter-toggles"]}>
        {LEVELS.map((level) => (
          <button
            key={level}
            className={classes(
              styles["filter-btn"],
              styles[level.toLowerCase()],
              shown.has(level) && styles.active,
            )}
            onClick={() => {
              onToggle(level);
            }}
          >
            {level}
          </button>
        ))}
      </div>
    </div>
  );
}

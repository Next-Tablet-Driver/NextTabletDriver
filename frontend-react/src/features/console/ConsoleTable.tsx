import { levelOf } from "@/features/console/logs";
import type { LogEntry } from "@/infrastructure/tauri/commands";
import styles from "@/features/console/ConsoleTable.module.css";

function levelColor(entry: LogEntry): string {
  switch (levelOf(entry)) {
    case "Info":
      return "var(--info)";
    case "Warn":
      return "var(--warning)";
    case "Error":
      return "var(--error)";
    default:
      return "var(--debug)";
  }
}

export function ConsoleTable({ filteredLogs }: { filteredLogs: readonly LogEntry[] }) {
  return (
    <div className={styles["table-container"]}>
      <table className={styles["logs-table"]}>
        <thead>
          <tr>
            <th className={styles["col-time"]}>Time</th>
            <th className={styles["col-level"]}>Level</th>
            <th className={styles["col-group"]}>Group</th>
            <th className={styles["col-msg"]}>Message</th>
          </tr>
        </thead>
      </table>
      <div className={styles["table-body-scroll"]}>
        <table className={`${styles["logs-table"] ?? ""} ${styles["body-table"] ?? ""}`}>
          <tbody>
            {filteredLogs.map((entry, index) => (
              <tr key={index}>
                <td className={styles["col-time"]}>{entry.time}</td>
                <td className={styles["col-level"]} style={{ color: levelColor(entry) }}>
                  {entry.level}
                </td>
                <td className={styles["col-group"]}>{entry.group}</td>
                <td className={styles["col-msg"]}>{entry.message}</td>
              </tr>
            ))}
            {filteredLogs.length === 0 && (
              <tr className={styles["empty-row"]}>
                <td colSpan={4}>No logs to display</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

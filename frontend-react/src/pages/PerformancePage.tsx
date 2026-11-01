import { TitleBar } from "@/shell/TitleBar";
import styles from "@/pages/PerformancePage.module.css";

export function PerformancePage() {
  return (
    <div className={styles["detached-window"]}>
      <TitleBar title="NextTabletDriver - Input Lag Analysis" />
      <div className={styles.content}>
        <h2>Input Lag Analysis</h2>
        <p>This page is running in a detached native Tauri window!</p>
        <p>Real-time performance metrics will be rendered here soon...</p>
      </div>
    </div>
  );
}

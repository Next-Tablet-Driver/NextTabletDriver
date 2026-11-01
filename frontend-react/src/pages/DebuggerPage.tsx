import { TitleBar } from "@/shell/TitleBar";
import styles from "@/pages/DebuggerPage.module.css";

export function DebuggerPage() {
  return (
    <div className={styles["detached-window"]}>
      <TitleBar title="NextTabletDriver - Debugger" />
      <div className={styles.content}>
        <h2>Tablet Debugger</h2>
        <p>This page is running in a detached native Tauri window!</p>
        <p>Real-time tablet data will be rendered here soon...</p>
      </div>
    </div>
  );
}

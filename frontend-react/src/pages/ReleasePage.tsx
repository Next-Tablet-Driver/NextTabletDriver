import { ReleaseCard } from "@/features/release/ReleaseCard";
import { useCommand } from "@/hooks/useCommand";
import { getReleases } from "@/infrastructure/tauri/commands";
import styles from "@/pages/ReleasePage.module.css";

/** What changed in each version of the driver, read from GitHub. */
export function ReleasePage() {
  const { state } = useCommand(getReleases);

  return (
    <div className={styles["release-container"]}>
      <div className={styles["page-header"]}>
        <h1 className={styles["page-title"]}>Release Notes</h1>
        <p className={styles["page-subtitle"]}>
          Discover what&apos;s new and improved in the driver.
        </p>
      </div>
      {state.status === "loading" && (
        <div className={styles["loading-state"]}>
          <div className={styles.spinner} />
          <span>Fetching releases from GitHub...</span>
        </div>
      )}
      {state.status === "error" && (
        <div className={styles["error-state"]}>Failed to load releases: {state.message}</div>
      )}
      {state.status === "ready" && (
        <div className={styles["releases-list"]}>
          {state.data.map((release, index) => (
            <ReleaseCard key={release.tag_name} release={release} isLatest={index === 0} />
          ))}
        </div>
      )}
    </div>
  );
}

import { useEffect, useState } from "react";
import { Minus, RefreshCcw, Square, X } from "lucide-react";
import iconUrl from "@/assets/icon.png";
import { log } from "@/infrastructure/log";
import { os, type Update } from "@/infrastructure/os";
import { windowControls } from "@/infrastructure/tauri/window";
import { Button } from "@/ui/Button";
import { classes } from "@/ui/classes";
import styles from "@/shell/TitleBar.module.css";

/** The window's own title bar: the window is undecorated, so this draws its controls. */
export function TitleBar({
  title = "NextTabletDriver",
  systemTrayOnMinimize = false,
}: {
  title?: string;
  systemTrayOnMinimize?: boolean;
}) {
  const [update, setUpdate] = useState<Update | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    os.checkForUpdate().then(setUpdate, (e: unknown) => {
      log.error("Failed to check for updates:", e);
    });
  }, []);

  const install = async (): Promise<void> => {
    if (update === null || isUpdating) {
      return;
    }
    try {
      setIsUpdating(true);
      let downloaded = 0;
      let contentLength = 0;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              setProgress(Math.round((downloaded / contentLength) * 100));
            }
            break;
          case "Finished":
            setProgress(100);
            break;
        }
      });
      await os.relaunch();
    } catch (e) {
      log.error("Update installation failed:", e);
      setIsUpdating(false);
    }
  };

  return (
    <div className={styles.titlebar} data-tauri-drag-region>
      <div className={styles["icon-container"]} data-tauri-drag-region>
        <img className={styles["app-icon"]} alt="App Icon" src={iconUrl} />
      </div>

      <div className={styles.title} data-tauri-drag-region>
        {title}
      </div>

      {update !== null && (
        <Button
          className="update-btn"
          disabled={isUpdating}
          variant="primary"
          onClick={() => void install()}
        >
          <RefreshCcw className={classes(isUpdating && "spinning")} size={13} strokeWidth={2.5} />
          <span>{isUpdating ? `Updating (${String(progress)}%)` : "Update"}</span>
        </Button>
      )}

      <div className={styles["window-controls"]}>
        <button
          className={styles.control}
          onClick={() =>
            void (systemTrayOnMinimize ? windowControls.hide() : windowControls.minimize())
          }
        >
          <Minus size={16} strokeWidth={1.8} />
        </button>
        <button className={styles.control} onClick={() => void windowControls.toggleMaximize()}>
          <Square size={12} strokeWidth={2} />
        </button>
        <button
          className={classes(styles.control, styles.close)}
          onClick={() => void windowControls.close()}
        >
          <X size={18} strokeWidth={1.8} />
        </button>
      </div>
    </div>
  );
}

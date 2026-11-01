import { ShieldAlert } from "lucide-react";
import type { UntrustedPlugin } from "@/infrastructure/tauri/commands";
import { Button } from "@/ui/Button";
import styles from "@/features/filters/UntrustedPlugins.module.css";

/** Libraries in the plugins folder that were not loaded because nobody approved them. */
export function UntrustedPlugins({
  plugins,
  onTrust,
}: {
  plugins: UntrustedPlugin[];
  onTrust: (sha256: string) => void;
}) {
  if (plugins.length === 0) {
    return null;
  }
  return (
    <div className={styles["untrusted-banner"]} role="alert">
      <div className={styles["banner-title"]}>
        <ShieldAlert size={18} />
        <strong>
          {plugins.length === 1
            ? "1 plugin was not loaded"
            : `${String(plugins.length)} plugins were not loaded`}
        </strong>
      </div>
      <p>
        These libraries are in the plugins folder but were not installed through NextTabletDriver.
        Plugins run native code with full access to your system, so they stay disabled until you
        approve them.
      </p>
      <ul>
        {plugins.map((plugin) => (
          <li key={`${plugin.file_name}:${plugin.sha256}`}>
            <span className={styles.file}>{plugin.file_name}</span>
            <code className={styles.hash} title={plugin.sha256}>
              {plugin.sha256.slice(0, 12)}…
            </code>
            <Button
              onClick={() => {
                onTrust(plugin.sha256);
              }}
            >
              Review and trust
            </Button>
          </li>
        ))}
      </ul>
    </div>
  );
}

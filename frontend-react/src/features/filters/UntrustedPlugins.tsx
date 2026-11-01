import { Alert } from "@getpapillon/papicons";
import { Button } from "@/components/base/buttons/button";
import type { UntrustedPlugin } from "@/infrastructure/tauri/commands";

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
    <section role="alert" className="flex flex-col gap-3 bg-warning-primary px-4 py-3">
      <div className="flex items-start gap-3">
        <Alert className="mt-0.5 size-5 shrink-0 text-fg-warning-primary" aria-hidden="true" />
        <div className="flex flex-col gap-0.5">
          <h2 className="text-sm font-semibold text-primary">
            {plugins.length === 1
              ? "1 plugin was not loaded"
              : `${String(plugins.length)} plugins were not loaded`}
          </h2>
          <p className="text-sm text-secondary">
            These libraries are in the plugins folder but were not installed through
            NextTabletDriver. Plugins run native code with full access to your system, so they stay
            disabled until you approve them.
          </p>
        </div>
      </div>
      <ul className="flex flex-col divide-y divide-warning">
        {plugins.map((plugin) => (
          <li
            key={`${plugin.file_name}:${plugin.sha256}`}
            className="flex flex-wrap items-center gap-3 py-2"
          >
            <span className="min-w-0 flex-1 truncate text-sm font-medium text-primary">
              {plugin.file_name}
            </span>
            <code className="text-xs text-tertiary" title={plugin.sha256}>
              {plugin.sha256.slice(0, 12)}…
            </code>
            <Button
              size="sm"
              color="secondary"
              onPress={() => {
                onTrust(plugin.sha256);
              }}
            >
              Review and trust
            </Button>
          </li>
        ))}
      </ul>
    </section>
  );
}

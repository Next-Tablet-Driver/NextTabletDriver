import { useAppStatus } from "@/app/app-status";
import { ButtonGroup, ButtonGroupItem } from "@/components/base/button-group/button-group";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import type { DriverMode } from "@/infrastructure/tauri/commands";

const MODES: { id: DriverMode; label: string }[] = [
  { id: "Absolute", label: "Absolute" },
  { id: "Relative", label: "Relative" },
];

/** The footer: the driver mode, the tablet, the profile (starred when unsaved) and the version. */
export function StatusBar() {
  const { version, profileName, tablet } = useAppStatus();
  const { config, isDirty } = useMappingSnapshot();
  const mapping = useMappingStore();

  return (
    <footer className="flex shrink-0 flex-wrap items-center justify-between gap-x-6 gap-y-2 border-t border-secondary bg-primary px-4 py-2 text-sm text-tertiary">
      <ButtonGroup
        aria-label="Driver mode"
        size="sm"
        selectedKeys={[config?.mode ?? "Absolute"]}
        disallowEmptySelection
        isDisabled={config === null}
        onSelectionChange={(keys) => {
          const [mode] = keys;
          if (mode !== undefined) {
            mapping.update((c) => ({ ...c, mode: mode as DriverMode }));
          }
        }}
      >
        {MODES.map((mode) => (
          <ButtonGroupItem key={mode.id} id={mode.id}>
            {mode.label}
          </ButtonGroupItem>
        ))}
      </ButtonGroup>
      <div className="flex flex-wrap items-center gap-x-5 gap-y-1">
        <span className="flex items-center gap-2 text-primary">
          <span
            aria-hidden="true"
            className={`size-2 rounded-full ${tablet.connected ? "bg-fg-success-primary" : "bg-fg-quaternary"}`}
          />
          {tablet.name}
        </span>
        <span>
          Profile:{" "}
          <strong className={isDirty ? "font-semibold text-primary italic" : "text-primary"}>
            {profileName}
            {isDirty ? "*" : ""}
          </strong>
        </span>
        <span className="tabular-nums">{version}</span>
      </div>
    </footer>
  );
}

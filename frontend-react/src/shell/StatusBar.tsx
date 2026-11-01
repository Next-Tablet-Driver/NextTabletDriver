import { useAppStatus } from "@/app/app-status";
import { Badge, BadgeWithDot } from "@/components/base/badges/badges";
import { Select } from "@/components/base/select/select";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import type { DriverMode } from "@/infrastructure/tauri/commands";

const MODES = [
  { id: "Absolute", label: "Absolute Mode" },
  { id: "Relative", label: "Relative Mode" },
];

/** The footer: the driver mode, the tablet, the profile (starred when unsaved) and the version. */
export function StatusBar() {
  const { version, profileName, tablet } = useAppStatus();
  const { config, isDirty } = useMappingSnapshot();
  const mapping = useMappingStore();

  return (
    <footer className="flex min-h-14 shrink-0 flex-wrap items-center justify-between gap-3 border-t border-secondary bg-primary px-4 py-2">
      <div className="w-44">
        <Select
          aria-label="Driver mode"
          size="sm"
          items={MODES}
          value={config?.mode ?? "Absolute"}
          isDisabled={config === null}
          onChange={(key) => {
            mapping.update((c) => ({ ...c, mode: String(key) as DriverMode }));
          }}
        >
          {(item) => <Select.Item {...item} />}
        </Select>
      </div>
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm text-tertiary">
        <BadgeWithDot type="pill-color" size="md" color={tablet.connected ? "success" : "gray"}>
          {tablet.name}
        </BadgeWithDot>
        <span>
          Profile:{" "}
          <strong className={isDirty ? "font-semibold text-primary italic" : "text-primary"}>
            {profileName}
            {isDirty ? "*" : ""}
          </strong>
        </span>
        <Badge type="pill-color" size="md" color="gray">
          {version}
        </Badge>
      </div>
    </footer>
  );
}

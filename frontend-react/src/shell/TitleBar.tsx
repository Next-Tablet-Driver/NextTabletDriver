import { useEffect, useState } from "react";
import {
  BarChart07,
  ChevronDown,
  Download01,
  File06,
  Folder,
  LinkExternal01,
  Minus,
  RefreshCcw01,
  Save01,
  Square,
  Tablet01,
  Upload01,
  XClose,
} from "@untitledui/icons";
import { profileActions } from "@/app/profile-actions";
import { checkForUpdatesInteractive } from "@/app/updates";
import { Button } from "@/components/base/buttons/button";
import { Dropdown } from "@/components/base/dropdown/dropdown";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { log } from "@/infrastructure/log";
import { os, type Update } from "@/infrastructure/os";
import { getPresets, type PresetInfo } from "@/infrastructure/tauri/commands";
import { windowControls } from "@/infrastructure/tauri/window";

const REPOSITORY = "https://github.com/Next-Tablet-Driver/NextTabletDriver";

const DETACHED_WINDOW = { decorations: false, transparent: true, minWidth: 400, minHeight: 300 };

function MenuTrigger({ label }: { label: string }) {
  return (
    <Button color="tertiary" size="sm" iconTrailing={ChevronDown}>
      {label}
    </Button>
  );
}

function FileMenu() {
  const mapping = useMappingStore();
  const actions = profileActions(mapping);
  const [presets, setPresets] = useState<PresetInfo[]>([]);

  useEffect(() => {
    getPresets().then(setPresets, log.error);
  }, []);

  return (
    <Dropdown.Root>
      <MenuTrigger label="File" />
      <Dropdown.Popover>
        <Dropdown.Menu>
          <Dropdown.Item
            id="open"
            icon={Folder}
            label="Load Settings..."
            onAction={() => void actions.open()}
          />
          <Dropdown.Item
            id="save"
            icon={Save01}
            label="Save Settings"
            onAction={() => void actions.save()}
          />
          <Dropdown.Item
            id="save-as"
            icon={Save01}
            label="Save Settings As..."
            onAction={() => void actions.saveAs()}
          />
          <Dropdown.Item
            id="reset"
            icon={RefreshCcw01}
            label="Reset to default"
            onAction={() => void actions.reset()}
          />
          <Dropdown.Separator />
          <Dropdown.Item
            id="export"
            icon={Upload01}
            label="Export .json"
            onAction={() => void actions.export()}
          />
          <Dropdown.Item
            id="import"
            icon={Download01}
            label="Import .json"
            onAction={() => void actions.import()}
          />
          <Dropdown.Item
            id="import-otd"
            icon={Download01}
            label="Import OTD Settings"
            onAction={() => void actions.importOtd()}
          />
          <Dropdown.Separator />
          <Dropdown.Section>
            <Dropdown.SectionHeader className="px-3 pt-1 pb-1 text-xs font-semibold text-tertiary">
              Presets
            </Dropdown.SectionHeader>
            {presets.length === 0 ? (
              <Dropdown.Item id="no-presets" isDisabled label="No presets" />
            ) : (
              presets.map((preset) => (
                <Dropdown.Item
                  key={preset.path}
                  id={preset.path}
                  icon={File06}
                  label={preset.name}
                  onAction={() => void actions.loadPreset(preset)}
                />
              ))
            )}
          </Dropdown.Section>
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown.Root>
  );
}

function TabletMenu() {
  return (
    <Dropdown.Root>
      <MenuTrigger label="Tablet" />
      <Dropdown.Popover>
        <Dropdown.Menu>
          <Dropdown.Item
            id="debugger"
            icon={Tablet01}
            label="Open Debugger"
            onAction={() => {
              os.openWindow("debugger", {
                url: "index.html?view=debugger",
                title: "Debugger",
                width: 600,
                height: 500,
                ...DETACHED_WINDOW,
              });
            }}
          />
          <Dropdown.Item
            id="performance"
            icon={BarChart07}
            label="Input Lag Analysis"
            onAction={() => {
              os.openWindow("performance", {
                url: "index.html?view=performance",
                title: "Input Lag Analysis",
                width: 500,
                height: 400,
                ...DETACHED_WINDOW,
              });
            }}
          />
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown.Root>
  );
}

function HelpMenu() {
  return (
    <Dropdown.Root>
      <MenuTrigger label="Help" />
      <Dropdown.Popover>
        <Dropdown.Menu>
          <Dropdown.Item
            id="github"
            icon={LinkExternal01}
            label="Github Repository"
            onAction={() => void os.openUrl(REPOSITORY)}
          />
          <Dropdown.Item
            id="updates"
            icon={RefreshCcw01}
            label="Check for Updates"
            onAction={() => void checkForUpdatesInteractive()}
          />
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown.Root>
  );
}

/** The update button appears when the updater found a newer version. */
function UpdateButton() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [progress, setProgress] = useState<number | null>(null);

  useEffect(() => {
    os.checkForUpdate().then(setUpdate, (e: unknown) => {
      log.error("Failed to check for updates:", e);
    });
  }, []);

  if (update === null) {
    return null;
  }

  const install = async (): Promise<void> => {
    let downloaded = 0;
    let total = 0;
    setProgress(0);
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          setProgress(total > 0 ? Math.round((downloaded / total) * 100) : 0);
        } else {
          setProgress(100);
        }
      });
      await os.relaunch();
    } catch (e) {
      log.error("Update installation failed:", e);
      setProgress(null);
    }
  };

  return (
    <Button
      color="primary"
      size="sm"
      iconLeading={RefreshCcw01}
      isLoading={progress !== null}
      showTextWhileLoading
      onPress={() => void install()}
    >
      {progress === null ? "Update" : `Updating (${String(progress)}%)`}
    </Button>
  );
}

/** The window's own title bar: the window is undecorated, so this draws its controls. */
export function TitleBar({ title }: { title: string }) {
  const { config } = useMappingSnapshot();
  const toTray = config?.system_tray_on_minimize ?? false;

  return (
    <header
      data-tauri-drag-region
      className="flex h-12 shrink-0 items-center gap-1 border-b border-secondary bg-primary pl-4"
    >
      <span
        data-tauri-drag-region
        className="mr-3 truncate text-sm font-semibold text-primary select-none"
      >
        {title}
      </span>
      <nav aria-label="Application menu" className="flex items-center">
        <FileMenu />
        <TabletMenu />
        <HelpMenu />
      </nav>
      <div data-tauri-drag-region className="h-full flex-1" />
      <UpdateButton />
      <div className="flex items-center pr-1">
        <Button
          color="tertiary"
          size="sm"
          aria-label="Minimize"
          iconLeading={Minus}
          onPress={() => void (toTray ? windowControls.hide() : windowControls.minimize())}
        />
        <Button
          color="tertiary"
          size="sm"
          aria-label="Maximize"
          iconLeading={Square}
          onPress={() => void windowControls.toggleMaximize()}
        />
        <Button
          color="tertiary-destructive"
          size="sm"
          aria-label="Close"
          iconLeading={XClose}
          onPress={() => void windowControls.close()}
        />
      </div>
    </header>
  );
}

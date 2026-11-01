import { refreshProfileName, setProfileName } from "@/app/app-status";
import type { MappingStore } from "@/features/mapping/mapping.store";
import { os } from "@/infrastructure/os";
import {
  exportProfile,
  importOtdProfile,
  loadProfile,
  resetConfig,
} from "@/infrastructure/tauri/commands";

const JSON_FILTER = [{ name: "JSON", extensions: ["json"] }];

function fileName(path: string): string | undefined {
  return path.split(/[\\/]/).pop();
}

function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Runs `action`; a failure is shown to the user in a native dialog, as `what` failing. */
async function reporting(what: string, action: () => Promise<void>): Promise<void> {
  try {
    await action();
  } catch (error) {
    await os.message(`Failed to ${what}: ${describe(error)}`);
  }
}

async function pickJson(): Promise<string | null> {
  const path = await os.openFileDialog({ multiple: false, filters: JSON_FILTER });
  return path === null || Array.isArray(path) ? null : path;
}

/** What the File menu does with the profile files. */
export function profileActions(mapping: MappingStore) {
  const loadFrom = async (
    path: string,
    load: (path: string) => Promise<void>,
    name = fileName(path),
  ): Promise<void> => {
    await load(path);
    if (name !== undefined) {
      setProfileName(name);
    }
    await mapping.load();
  };

  return {
    async open(): Promise<void> {
      const path = await pickJson();
      if (path !== null) {
        await reporting("load settings", () => loadFrom(path, loadProfile));
      }
    },

    async import(): Promise<void> {
      const path = await pickJson();
      if (path !== null) {
        await reporting("import", () => loadFrom(path, loadProfile));
      }
    },

    async importOtd(): Promise<void> {
      const path = await pickJson();
      if (path !== null) {
        await reporting("import OTD settings", () => loadFrom(path, importOtdProfile));
      }
    },

    async loadPreset(preset: { name: string; path: string }): Promise<void> {
      await reporting("load preset", () => loadFrom(preset.path, loadProfile, preset.name));
    },

    async save(): Promise<void> {
      await mapping.save();
    },

    async saveAs(): Promise<void> {
      const path = await os.saveFileDialog({ filters: JSON_FILTER });
      if (path !== null) {
        await reporting("save settings as", () => exportProfile(path));
      }
    },

    async export(): Promise<void> {
      const path = await os.saveFileDialog({ filters: JSON_FILTER });
      if (path !== null) {
        await reporting("export", () => exportProfile(path));
      }
    },

    async reset(): Promise<void> {
      await reporting("reset config", async () => {
        await resetConfig();
        setProfileName("Default Profile");
        await mapping.load();
      });
    },

    refreshName: refreshProfileName,
  };
}

import { log } from "@/infrastructure/log";
import {
  getCoreVersion,
  getCurrentProfileName,
  getTabletStatus,
  type TabletStatus,
} from "@/infrastructure/tauri/commands";
import { Store, useStore } from "@/lib/store";

export interface AppStatus {
  version: string;
  profileName: string;
  tablet: TabletStatus;
}

const NO_TABLET: TabletStatus = {
  connected: false,
  name: "No Tablet Detected",
  width: 0,
  height: 0,
  max_x: null,
  max_y: null,
  pen_button_count: 0,
  aux_button_count: 0,
};

/** What the title bar and the status bar show; refreshed by the backend's events. */
export const appStatus = new Store<AppStatus>({
  version: "",
  profileName: "Default Profile",
  tablet: NO_TABLET,
});

export function useAppStatus(): AppStatus {
  return useStore(appStatus);
}

export async function refreshTablet(): Promise<void> {
  try {
    const tablet = await getTabletStatus();
    appStatus.update((s) => ({ ...s, tablet }));
  } catch (e) {
    log.error("Failed to read the tablet status:", e);
  }
}

export async function refreshProfileName(): Promise<void> {
  try {
    const profileName = await getCurrentProfileName();
    appStatus.update((s) => ({ ...s, profileName }));
  } catch (e) {
    log.error("Failed to read the profile name:", e);
  }
}

/** Reads the version, the tablet and the profile name once at start-up. */
export async function loadAppStatus(): Promise<void> {
  try {
    const [version, tablet, profileName] = await Promise.all([
      getCoreVersion(),
      getTabletStatus(),
      getCurrentProfileName(),
    ]);
    appStatus.set({ version, tablet, profileName });
  } catch (e) {
    log.error("Failed to call Tauri commands:", e);
  }
}

export function setProfileName(profileName: string): void {
  appStatus.update((s) => ({ ...s, profileName }));
}

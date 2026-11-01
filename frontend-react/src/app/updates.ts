import { os } from "@/infrastructure/os";

/** Asks the user about an update and installs it; the same flow for the menu and the tray. */
export async function checkForUpdatesInteractive(): Promise<void> {
  try {
    const update = await os.checkForUpdate();
    if (update === null) {
      await os.message("You are already on the latest version.");
      return;
    }
    const yes = await os.confirm(
      `Update v${update.version} is available! Do you want to download and install it now?`,
    );
    if (yes) {
      await update.downloadAndInstall();
      await os.relaunch();
    }
  } catch (error) {
    await os.message(`Failed to check for updates: ${String(error)}`);
  }
}

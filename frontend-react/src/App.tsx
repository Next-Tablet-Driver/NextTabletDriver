import { useEffect } from "react";
import { loadAppStatus, refreshProfileName, refreshTablet, useAppStatus } from "@/app/app-status";
import { useHashPage } from "@/app/useHashPage";
import { checkForUpdatesInteractive } from "@/app/updates";
import {
  MappingProvider,
  useMappingSnapshot,
  useMappingStore,
} from "@/features/mapping/MappingProvider";
import { PageView } from "@/pages/PageView";
import { events } from "@/infrastructure/tauri/events";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { Sidebar } from "@/shell/Sidebar";
import { StatusBar } from "@/shell/StatusBar";
import { TitleBar } from "@/shell/TitleBar";
import { useTheme } from "@/theme/useTheme";

// Stable functions: a new one on every render would subscribe again every time. These events
// carry no data, so the subscription gives `undefined` to the handler.
type NoData = (data: undefined) => void;
const onDeviceChanged = (callback: NoData) =>
  events.onDeviceChanged(() => {
    callback(undefined);
  });
const onProfileLoaded = (callback: NoData) =>
  events.onProfileLoaded(() => {
    callback(undefined);
  });
const onNavigateToUpdates = (callback: NoData) =>
  events.onNavigateToUpdates(() => {
    callback(undefined);
  });

function MainWindow() {
  const page = useHashPage();
  const mapping = useMappingStore();
  const { config } = useMappingSnapshot();
  const { version } = useAppStatus();

  useTheme(config?.theme ?? "System");

  useEffect(() => {
    void loadAppStatus();
  }, []);

  useTauriEvent(onDeviceChanged, () => {
    void refreshTablet();
  });
  useTauriEvent(onProfileLoaded, () => {
    void refreshProfileName();
    void mapping.load();
  });
  useTauriEvent(onNavigateToUpdates, () => {
    void checkForUpdatesInteractive();
  });

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-secondary">
      <TitleBar title={`NextTabletDriver ${version}`} />
      <div className="flex min-h-0 flex-1">
        <Sidebar current={page} />
        <main className="min-w-0 flex-1 overflow-y-auto bg-secondary" id="content">
          <PageView page={page} />
        </main>
      </div>
      <StatusBar />
    </div>
  );
}

export default function App() {
  return (
    <MappingProvider>
      <MainWindow />
    </MappingProvider>
  );
}

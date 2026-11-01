import { useEffect } from "react";
import { loadAppStatus, refreshProfileName, refreshTablet, useAppStatus } from "@/app/app-status";
import { checkForUpdatesInteractive } from "@/app/updates";
import { useHashPage } from "@/app/useHashPage";
import {
  MappingProvider,
  useMappingSnapshot,
  useMappingStore,
} from "@/features/mapping/MappingProvider";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { events } from "@/infrastructure/tauri/events";
import { DebuggerPage } from "@/pages/DebuggerPage";
import { PageView } from "@/pages/PageView";
import { PerformancePage } from "@/pages/PerformancePage";
import { Footer } from "@/shell/Footer";
import { MenuBar } from "@/shell/MenuBar";
import { TabBar } from "@/shell/TabBar";
import { TitleBar } from "@/shell/TitleBar";
import { refreshUserThemes, themeStore } from "@/theme";
import styles from "@/App.module.css";

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

/** The configuration is the source of truth: the theme follows it. */
function useThemeFromConfig(): void {
  const { config } = useMappingSnapshot();
  const selected = config?.theme;

  useEffect(() => {
    themeStore.start();
    void refreshUserThemes();
    return () => {
      themeStore.stop();
    };
  }, []);

  useEffect(() => {
    if (selected !== undefined && selected !== "") {
      themeStore.select(selected);
    }
  }, [selected]);
}

function MainWindow() {
  const page = useHashPage();
  const mapping = useMappingStore();
  const { config } = useMappingSnapshot();
  const { version } = useAppStatus();

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
    <main id="app">
      <TitleBar
        title={`NextTabletDriver ${version}`}
        systemTrayOnMinimize={config?.system_tray_on_minimize ?? false}
      />
      <MenuBar />
      <TabBar active={page} />
      <div className={`${styles["content-area"] ?? ""} content-area`}>
        <PageView page={page} />
      </div>
      <Footer />
    </main>
  );
}

/** The windows opened from the Tablet menu are the same page with `?view=` in the address. */
function Window() {
  useThemeFromConfig();
  const view = new URLSearchParams(window.location.search).get("view");
  if (view === "debugger") {
    return <DebuggerPage />;
  }
  if (view === "performance") {
    return <PerformancePage />;
  }
  return <MainWindow />;
}

export default function App() {
  return (
    <MappingProvider>
      <Window />
    </MappingProvider>
  );
}

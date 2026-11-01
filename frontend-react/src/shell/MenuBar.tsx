import { useEffect, useRef, useState, type MouseEvent } from "react";
import {
  Bug,
  BugPlay,
  ChartNoAxesCombined,
  FileDown,
  FileUp,
  Folder,
  Info,
  Link,
  List,
  RefreshCcw,
  RotateCcw,
  Save,
  Smartphone,
} from "lucide-react";
import { profileActions } from "@/app/profile-actions";
import { checkForUpdatesInteractive } from "@/app/updates";
import { useMappingStore } from "@/features/mapping/MappingProvider";
import { log } from "@/infrastructure/log";
import { os } from "@/infrastructure/os";
import { getCoreVersion, getPresets, type PresetInfo } from "@/infrastructure/tauri/commands";
import { classes } from "@/ui/classes";
import { DropdownItem } from "@/ui/DropdownItem";
import { DropdownMenu } from "@/ui/DropdownMenu";
import { DropdownSeparator } from "@/ui/DropdownSeparator";
import styles from "@/shell/MenuBar.module.css";

const REPOSITORY = "https://github.com/Next-Tablet-Driver/NextTabletDriver";
const SUBMENU_CLOSE_DELAY_MS = 300;

/** The File, Tablet and Help menus under the title bar. */
export function MenuBar() {
  const mapping = useMappingStore();
  const actions = profileActions(mapping);
  const [openMenu, setOpenMenu] = useState<string | null>(null);
  const [openSubmenu, setOpenSubmenu] = useState<string | null>(null);
  const [presets, setPresets] = useState<PresetInfo[]>([]);
  const submenuTimer = useRef<number | undefined>(undefined);

  useEffect(() => {
    getPresets().then(setPresets, (e: unknown) => {
      log.error("Failed to load presets:", e);
    });
  }, []);

  const closeMenu = (): void => {
    setOpenMenu(null);
    setOpenSubmenu(null);
    window.clearTimeout(submenuTimer.current);
  };

  // A click anywhere else closes the menu.
  useEffect(() => {
    window.addEventListener("click", closeMenu);
    return () => {
      window.removeEventListener("click", closeMenu);
    };
  }, []);

  const toggleMenu = (menu: string, event: MouseEvent): void => {
    event.stopPropagation();
    setOpenMenu((current) => (current === menu ? null : menu));
  };
  const enterSubmenu = (menu: string): void => {
    window.clearTimeout(submenuTimer.current);
    setOpenSubmenu(menu);
  };
  const leaveSubmenu = (): void => {
    submenuTimer.current = window.setTimeout(() => {
      setOpenSubmenu(null);
    }, SUBMENU_CLOSE_DELAY_MS);
  };
  /** Closes the menu, then runs the action. */
  const run = (action: () => Promise<void> | void) => (): void => {
    closeMenu();
    void action();
  };

  return (
    <div className={styles["menu-bar"]}>
      <div className={styles["menu-container"]}>
        <button
          className={classes(styles["menu-item"], openMenu === "file" && styles.active)}
          onClick={(e) => {
            toggleMenu("file", e);
          }}
        >
          <Folder size={16} />
          File
        </button>
        {openMenu === "file" && (
          <DropdownMenu>
            <DropdownItem onClick={run(actions.open)}>
              <FileDown size={16} />
              Load Settings...
            </DropdownItem>
            <DropdownItem onClick={run(actions.save)}>
              <Save size={16} />
              Save Settings
            </DropdownItem>
            <DropdownItem onClick={run(actions.saveAs)}>
              <Save size={16} />
              Save Settings As...
            </DropdownItem>
            <DropdownItem onClick={run(actions.reset)}>
              <RotateCcw size={16} />
              Reset to default
            </DropdownItem>
            <DropdownSeparator />
            <DropdownItem onClick={run(actions.export)}>
              <FileUp size={16} />
              Export .json
            </DropdownItem>
            <DropdownItem onClick={run(actions.import)}>
              <FileDown size={16} />
              Import .json
            </DropdownItem>
            <DropdownItem onClick={run(actions.importOtd)}>
              <FileDown size={16} />
              Import OTD Settings
            </DropdownItem>
            <DropdownSeparator />
            <DropdownItem
              hasSubmenu
              onMouseEnter={() => {
                enterSubmenu("presets");
              }}
              onMouseLeave={leaveSubmenu}
            >
              <List size={16} />
              Presets
              {openSubmenu === "presets" && (
                <DropdownMenu placement="right-start">
                  {presets.length === 0 ? (
                    <DropdownItem disabled>
                      <span style={{ fontStyle: "italic" }}>No presets</span>
                    </DropdownItem>
                  ) : (
                    presets.map((preset) => (
                      <DropdownItem
                        key={preset.path}
                        onClick={run(() => actions.loadPreset(preset))}
                      >
                        {preset.name}
                      </DropdownItem>
                    ))
                  )}
                </DropdownMenu>
              )}
            </DropdownItem>
          </DropdownMenu>
        )}
      </div>

      <div className={styles["menu-container"]}>
        <button
          className={classes(styles["menu-item"], openMenu === "tablet" && styles.active)}
          onClick={(e) => {
            toggleMenu("tablet", e);
          }}
        >
          <Smartphone size={16} />
          Tablet
        </button>
        {openMenu === "tablet" && (
          <DropdownMenu>
            <DropdownItem
              onClick={run(() => {
                os.openWindow("debugger", {
                  url: "index.html?view=debugger",
                  title: "Debugger",
                  width: 600,
                  height: 500,
                  decorations: false,
                  transparent: true,
                  minWidth: 400,
                  minHeight: 300,
                });
              })}
            >
              <BugPlay size={16} />
              Open Debugger
            </DropdownItem>
            <DropdownItem
              onClick={run(() => {
                os.openWindow("performance", {
                  url: "index.html?view=performance",
                  title: "Input Lag Analysis",
                  width: 500,
                  height: 400,
                  decorations: false,
                  transparent: true,
                  minWidth: 400,
                  minHeight: 300,
                });
              })}
            >
              <ChartNoAxesCombined size={16} />
              Input Lag Analysis
            </DropdownItem>
          </DropdownMenu>
        )}
      </div>

      <div className={styles["menu-container"]}>
        <button
          className={classes(styles["menu-item"], openMenu === "help" && styles.active)}
          onClick={(e) => {
            toggleMenu("help", e);
          }}
        >
          <Info size={16} />
          Help
        </button>
        {openMenu === "help" && (
          <DropdownMenu>
            <DropdownItem onClick={run(() => os.openUrl(REPOSITORY))}>
              <Link size={16} />
              Github Repository
            </DropdownItem>
            <DropdownItem
              hasSubmenu
              onMouseEnter={() => {
                enterSubmenu("bug");
              }}
              onMouseLeave={leaveSubmenu}
            >
              <Bug size={16} /> Report Bug / Suggest Feature
              {openSubmenu === "bug" && (
                <DropdownMenu placement="right-start">
                  <DropdownItem
                    onClick={run(async () => {
                      const version = await getCoreVersion();
                      await os.openUrl(
                        `${REPOSITORY}/issues/new?template=bug_report.yml&ntd_version=${version}`,
                      );
                    })}
                  >
                    Report a Bug
                  </DropdownItem>
                  <DropdownItem
                    onClick={run(() =>
                      os.openUrl(`${REPOSITORY}/issues/new?template=feature_request.yml`),
                    )}
                  >
                    Suggest a Feature
                  </DropdownItem>
                </DropdownMenu>
              )}
            </DropdownItem>
            <DropdownItem onClick={run(checkForUpdatesInteractive)}>
              <RefreshCcw size={16} />
              Check for Update
            </DropdownItem>
          </DropdownMenu>
        )}
      </div>
    </div>
  );
}

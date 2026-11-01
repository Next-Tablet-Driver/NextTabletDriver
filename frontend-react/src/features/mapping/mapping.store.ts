import { log } from "@/infrastructure/log";
import { getConfig, saveConfig, setConfig, type Config } from "@/infrastructure/tauri/commands";
import { Store } from "@/lib/store";

/** How long the backend waits for the user to stop editing before it receives the change. */
export const APPLY_DELAY_MS = 400;
const HISTORY_MAX_SIZE = 50;

export interface MappingSnapshot {
  /** The configuration being edited; `null` until the first load. */
  config: Config | null;
  /** The profile differs from the one on disk. */
  isDirty: boolean;
  canUndo: boolean;
  canRedo: boolean;
}

const EMPTY: MappingSnapshot = { config: null, isDirty: false, canUndo: false, canRedo: false };

/**
 * The configuration the interface edits, with its undo history.
 *
 * Every edit replaces `config` at once (so the screen answers immediately) and sends it to the
 * backend after `APPLY_DELAY_MS` of quiet. "Dirty" means different from what was last saved to
 * the profile file.
 */
export class MappingStore {
  readonly state = new Store<MappingSnapshot>(EMPTY);

  #history: string[] = [];
  #index = -1;
  #saved = "";
  #timer: ReturnType<typeof setTimeout> | undefined;

  /** Reads the configuration from the backend and starts a fresh history. */
  async load(): Promise<void> {
    try {
      const config = await getConfig();
      const text = JSON.stringify(config);
      clearTimeout(this.#timer);
      this.#history = [text];
      this.#index = 0;
      this.#saved = text;
      this.#publish(config);
    } catch (e) {
      log.error("Failed to load config", e);
    }
  }

  /** Edits the configuration. */
  update(change: (config: Config) => Config): void {
    const current = this.state.get().config;
    if (current === null) {
      return;
    }
    const next = change(current);
    // An edit that changes nothing (a value clamped back to what it was) is not an edit.
    if (JSON.stringify(next) === JSON.stringify(current)) {
      return;
    }
    this.#publish(next);
    clearTimeout(this.#timer);
    this.#timer = setTimeout(() => {
      this.#commit();
    }, APPLY_DELAY_MS);
  }

  /** Sends a pending edit to the backend now. */
  flush(): void {
    if (this.#timer !== undefined) {
      clearTimeout(this.#timer);
      this.#commit();
    }
  }

  undo(): void {
    this.#move(-1);
  }

  redo(): void {
    this.#move(1);
  }

  /** Goes back to the configuration that was loaded. */
  revert(): void {
    this.#move(-this.#index);
  }

  /** Writes the profile file. */
  async save(): Promise<void> {
    this.flush();
    try {
      await saveConfig();
      this.#saved = this.#history[this.#index] ?? this.#saved;
      this.#publish(this.state.get().config);
    } catch (e) {
      log.error("Failed to save the profile", e);
    }
  }

  #commit(): void {
    this.#timer = undefined;
    const config = this.state.get().config;
    if (config === null) {
      return;
    }
    setConfig(config).catch(log.error);
    const text = JSON.stringify(config);
    if (text !== this.#history[this.#index]) {
      this.#history = [...this.#history.slice(0, this.#index + 1), text];
      this.#index = this.#history.length - 1;
      if (this.#history.length > HISTORY_MAX_SIZE) {
        this.#history.shift();
        this.#index--;
      }
    }
    this.#publish(config);
  }

  #move(step: number): void {
    const target = this.#index + step;
    const text = this.#history[target];
    if (text === undefined) {
      return;
    }
    clearTimeout(this.#timer);
    this.#timer = undefined;
    this.#index = target;
    const config = JSON.parse(text) as Config;
    setConfig(config).catch(log.error);
    this.#publish(config);
  }

  #publish(config: Config | null): void {
    const text = config === null ? "" : JSON.stringify(config);
    this.state.set({
      config,
      isDirty: config !== null && text !== this.#saved,
      canUndo: this.#index > 0,
      canRedo: this.#index >= 0 && this.#index < this.#history.length - 1,
    });
  }
}

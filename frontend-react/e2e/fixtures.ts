import { test as base, expect, type Locator, type Page } from "@playwright/test";

export interface Invocation {
  command: string;
  args: Record<string, unknown>;
}

export const TABS = [
  "Output",
  "Filters",
  "Pen Settings",
  "Console",
  "Settings",
  "Release",
  "Credits",
] as const;
export type TabName = (typeof TABS)[number];

/** Thin helper around the page, with what every screen test needs. */
export class App {
  constructor(readonly page: Page) {}

  /** Opens the app against the Tauri mock and waits until the config has been loaded. */
  async open(query = ""): Promise<void> {
    await this.page.goto(`/?mock${query}`);
    await expect(this.page.getByText("Profile:")).toBeVisible();
    await expect(this.content.getByRole("heading", { level: 1 })).toBeVisible();
  }

  async goTo(tab: TabName): Promise<void> {
    await this.page.getByRole("tab", { name: tab, exact: true }).click();
  }

  /** Types a value and leaves the field, which is what commits a number input in this app. */
  async enter(field: Locator, value: string): Promise<void> {
    await field.fill(value);
    await field.press("Tab");
  }

  /** A switch of the current page: its visible track is what a person clicks. */
  async toggle(name: string): Promise<void> {
    await this.content
      .locator("label", { has: this.page.getByRole("switch", { name, exact: true }) })
      .click();
  }

  /** A text or number field of the current page, by its label. */
  field(label: string): Locator {
    return this.content.getByRole("textbox", { name: label, exact: true });
  }

  get content(): Locator {
    return this.page.locator("#content");
  }

  get footer(): Locator {
    return this.page.locator("footer");
  }

  /** The IPC calls made so far, optionally only those of one command. */
  async invocations(command?: string): Promise<Invocation[]> {
    const all = await this.page.evaluate(() => window.__ntdInvocations ?? []);
    return command === undefined ? all : all.filter((call) => call.command === command);
  }

  /** The configuration of the most recent `set_config` call (the saves are debounced). */
  async lastSavedConfig(): Promise<Record<string, unknown>> {
    const last = (await this.invocations("set_config")).at(-1);
    return (last?.args.newConfig ?? {}) as Record<string, unknown>;
  }

  /** Waits for the debounced save and returns what the UI sent once `ready` accepts it. */
  async savedConfigWhere(
    ready: (config: Record<string, unknown>) => boolean,
  ): Promise<Record<string, unknown>> {
    await expect.poll(async () => ready(await this.lastSavedConfig())).toBe(true);
    return this.lastSavedConfig();
  }
}

interface Fixtures {
  app: App;
  /** Fails the test when the page logs an error or throws, whatever the screen under test. */
  noRuntimeErrors: undefined;
}

export const test = base.extend<Fixtures>({
  noRuntimeErrors: [
    async ({ page }, use) => {
      const problems: string[] = [];
      page.on("console", (message) => {
        if (message.type() === "error") problems.push(`console.error: ${message.text()}`);
      });
      page.on("pageerror", (error) => {
        problems.push(`uncaught: ${error.message}`);
      });
      await use(undefined);
      expect(problems, "the page must not log errors").toEqual([]);
    },
    { auto: true },
  ],
  app: async ({ page }, use) => {
    await use(new App(page));
  },
});

export { expect };

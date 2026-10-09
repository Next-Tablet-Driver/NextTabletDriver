import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";
import type { Config } from "../../../infrastructure/tauri/commands";

const listThemes = vi.fn();
const importTheme = vi.fn();
const deleteTheme = vi.fn();
const openThemesFolder = vi.fn();
const confirmDialog = vi.fn();

vi.mock("../../../infrastructure/tauri/commands", () => ({
    listThemes: () => listThemes() as unknown,
    importTheme: () => importTheme() as unknown,
    deleteTheme: (id: string) => deleteTheme(id) as unknown,
    openThemesFolder: () => openThemesFolder() as unknown,
    getConfig: vi.fn(),
    setConfig: vi.fn(() => Promise.resolve()),
    saveConfig: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ confirm: (m: string) => confirmDialog(m) as unknown }));
vi.mock("@tauri-apps/plugin-shell", () => ({ open: vi.fn() }));

import ThemeSettings from "../ThemeSettings.svelte";
import { themeStore } from "../../../theme";
import { MappingState } from "../../mapping/mapping-state.svelte";

function themeFile(name: string, id: string): { id: string; file_name: string; content: string } {
    return {
        id,
        file_name: `${id}.json`,
        content: JSON.stringify({ schema: 1, metadata: { name }, base: "dark", tokens: { accent: "#ff007f" } }),
    };
}

function makeMapping(theme = "Dark"): MappingState {
    const mapping = new MappingState();
    mapping.config = { theme } as Config;
    mapping.isLoaded = true;
    return mapping;
}

describe("ThemeSettings", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        listThemes.mockResolvedValue([]);
        confirmDialog.mockResolvedValue(true);
        themeStore.setUserThemes([]);
        themeStore.select("Dark");
    });

    it("loads the user themes when opened and lists problems of broken files", async () => {
        listThemes.mockResolvedValue([
            themeFile("Neon Night", "neon-night"),
            { id: "broken", file_name: "broken.json", content: JSON.stringify({ schema: 1, metadata: { name: "B" }, tokens: { accent: "red" } }) },
        ]);
        render(ThemeSettings, { props: { mapping: makeMapping() } });

        await waitFor(() => { expect(themeStore.userThemes).toHaveLength(2); });
        expect(await screen.findByText("Themes that could not be loaded")).toBeInTheDocument();
        expect(screen.getByText("broken.json")).toBeInTheDocument();
        expect(screen.getByText("tokens.accent")).toBeInTheDocument();
    });

    it("imports a theme, selects it and records the choice in the configuration", async () => {
        const mapping = makeMapping();
        importTheme.mockResolvedValue(themeFile("Neon Night", "neon-night"));
        listThemes.mockResolvedValueOnce([]).mockResolvedValue([themeFile("Neon Night", "neon-night")]);
        render(ThemeSettings, { props: { mapping } });

        await fireEvent.click(await screen.findByRole("button", { name: /import theme/i }));

        await waitFor(() => { expect(mapping.config?.theme).toBe("custom:neon-night"); });
        expect(await screen.findByRole("status")).toHaveTextContent('Imported "Neon Night"');
    });

    it("does nothing when the import dialog is cancelled", async () => {
        const mapping = makeMapping();
        importTheme.mockResolvedValue(null);
        render(ThemeSettings, { props: { mapping } });

        await fireEvent.click(await screen.findByRole("button", { name: /import theme/i }));
        await waitFor(() => { expect(importTheme).toHaveBeenCalled(); });
        expect(mapping.config?.theme).toBe("Dark");
    });

    it("only offers Delete for a user theme, asks first, and falls back to Dark", async () => {
        const mapping = makeMapping("custom:neon-night");
        listThemes.mockResolvedValueOnce([themeFile("Neon Night", "neon-night")]).mockResolvedValue([]);
        themeStore.select("custom:neon-night");
        render(ThemeSettings, { props: { mapping } });
        await waitFor(() => { expect(themeStore.userThemes).toHaveLength(1); });

        const remove = await screen.findByRole("button", { name: /delete/i });
        confirmDialog.mockResolvedValueOnce(false);
        await fireEvent.click(remove);
        expect(deleteTheme).not.toHaveBeenCalled();

        await fireEvent.click(remove);
        await waitFor(() => { expect(deleteTheme).toHaveBeenCalledWith("neon-night"); });
        await waitFor(() => { expect(mapping.config?.theme).toBe("Dark"); });
    });

    it("does not show Delete for a built-in theme", async () => {
        render(ThemeSettings, { props: { mapping: makeMapping("Light") } });
        await screen.findByRole("button", { name: /import theme/i });
        expect(screen.queryByRole("button", { name: /delete/i })).toBeNull();
    });
});

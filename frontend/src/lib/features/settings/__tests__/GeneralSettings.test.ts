import { describe, it, expect, vi, type Mock } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import type { MappingState } from "../../mapping/mapping-state.svelte";
import GeneralSettings from "../GeneralSettings.svelte";
import LanguageSettings from "../LanguageSettings.svelte";

function fakeMapping(config: Record<string, unknown> | null): { mapping: MappingState; markDirty: Mock } {
    const markDirty = vi.fn();
    return { mapping: { config, markDirty } as unknown as MappingState, markDirty };
}

describe("GeneralSettings", () => {
    const config = { run_at_startup: true, system_tray_on_minimize: false, force_high_resolution_timer: true };

    it("reflects the startup, tray and timer options", () => {
        const { mapping } = fakeMapping({ ...config });
        render(GeneralSettings, { props: { mapping } });
        expect(screen.getByText("General Settings")).toBeInTheDocument();
        expect(screen.getByLabelText("Run at startup")).toBeChecked();
        expect(screen.getByLabelText("System Tray when Minimize")).not.toBeChecked();
        expect(screen.getByLabelText("Force High Resolution Timer (0.5ms)")).toBeChecked();
    });

    it("marks the profile dirty when an option changes", async () => {
        const { mapping, markDirty } = fakeMapping({ ...config });
        render(GeneralSettings, { props: { mapping } });
        await fireEvent.click(screen.getByLabelText("System Tray when Minimize"));
        expect(markDirty).toHaveBeenCalled();
    });

    it("lists the anonymous statistics option, enabled by default", () => {
        const { mapping } = fakeMapping({ ...config });
        render(GeneralSettings, { props: { mapping } });
        expect(screen.getByLabelText("Enable Anonymous Usage Statistics")).toBeChecked();
    });

    it("shows only the header until the config is loaded", () => {
        const { mapping } = fakeMapping(null);
        render(GeneralSettings, { props: { mapping } });
        expect(screen.getByText("General Settings")).toBeInTheDocument();
        expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    });
});

describe("LanguageSettings", () => {
    it("shows the language section once the config is loaded", () => {
        const { mapping } = fakeMapping({ language: "English" });
        render(LanguageSettings, { props: { mapping } });
        expect(screen.getByText("Language")).toBeInTheDocument();
        expect(screen.getByText("Interface Language")).toBeInTheDocument();
    });

    it("shows only the header until the config is loaded", () => {
        const { mapping } = fakeMapping(null);
        render(LanguageSettings, { props: { mapping } });
        expect(screen.getByText("Language")).toBeInTheDocument();
        expect(screen.queryByText("Interface Language")).toBeNull();
    });
});

import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import { fakeMapping } from "../../../test-support/fakeMapping";
import GeneralSettings from "../GeneralSettings.svelte";
import LanguageSettings from "../LanguageSettings.svelte";

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

    it("writes the changed option and marks the profile dirty", async () => {
        const { mapping, markDirty, config: written } = fakeMapping({ ...config });
        render(GeneralSettings, { props: { mapping } });
        await fireEvent.click(screen.getByLabelText("System Tray when Minimize"));
        expect(written).toEqual({ ...config, system_tray_on_minimize: true });
        expect(markDirty).toHaveBeenCalledOnce();
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

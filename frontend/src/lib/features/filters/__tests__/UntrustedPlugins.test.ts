import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import UntrustedPlugins from "../UntrustedPlugins.svelte";

const plugin = { file_name: "sketchy.dll", sha256: "a".repeat(64) };

describe("UntrustedPlugins", () => {
    it("renders nothing when every plugin is trusted", () => {
        render(UntrustedPlugins, { props: { plugins: [], onTrust: vi.fn() } });
        expect(screen.queryByRole("alert")).toBeNull();
    });

    it("lists pending libraries with a shortened hash", () => {
        render(UntrustedPlugins, { props: { plugins: [plugin], onTrust: vi.fn() } });
        expect(screen.getByRole("alert")).toBeInTheDocument();
        expect(screen.getByText("sketchy.dll")).toBeInTheDocument();
        expect(screen.getByText("aaaaaaaaaaaa…")).toBeInTheDocument();
    });

    it("asks to trust exactly the reviewed hash", async () => {
        const onTrust = vi.fn();
        render(UntrustedPlugins, { props: { plugins: [plugin], onTrust } });
        await fireEvent.click(screen.getByRole("button", { name: /review and trust/i }));
        expect(onTrust).toHaveBeenCalledWith(plugin.sha256);
    });

    it("lists two files with the same hash, such as a copy of a library", () => {
        const copy = { file_name: "sketchy - Copy.dll", sha256: plugin.sha256 };
        render(UntrustedPlugins, { props: { plugins: [plugin, copy], onTrust: vi.fn() } });
        expect(screen.getByText("sketchy.dll")).toBeInTheDocument();
        expect(screen.getByText("sketchy - Copy.dll")).toBeInTheDocument();
    });

    it("pluralises the title", () => {
        const other = { file_name: "b.dll", sha256: "b".repeat(64) };
        render(UntrustedPlugins, { props: { plugins: [plugin, other], onTrust: vi.fn() } });
        expect(screen.getByText("2 plugins were not loaded")).toBeInTheDocument();
    });
});

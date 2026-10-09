import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";
import Footer from "../Footer.svelte";

describe("Footer", () => {
    it("shows sensible defaults before a tablet is connected", () => {
        const { container } = render(Footer);
        expect(container.querySelector(".profile")).toHaveTextContent("Profile: Default");
        expect(container.querySelector(".version")).toHaveTextContent("Unknown");
        expect(container).toHaveTextContent("No Tablet Detected");
    });

    it("shows the profile and the app version", () => {
        const { container } = render(Footer, {
            props: { tabletName: "Wacom CTL-472", profileName: "osu!", version: "v2.0.0" },
        });
        expect(container.querySelector(".profile")).toHaveTextContent("Profile: osu!");
        expect(container.querySelector(".profile")).not.toHaveClass("dirty");
        expect(screen.getByText("v2.0.0")).toBeInTheDocument();
        expect(container).toHaveTextContent("Wacom CTL-472");
    });

    it("flags unsaved changes with an asterisk and italics", () => {
        const { container } = render(Footer, { props: { profileName: "osu!", isDirty: true } });
        expect(container.querySelector(".profile")).toHaveClass("dirty");
        const name = container.querySelector(".profile strong");
        expect(name).toHaveTextContent("osu!*");
        expect(name).toHaveStyle({ fontStyle: "italic" });
    });
});

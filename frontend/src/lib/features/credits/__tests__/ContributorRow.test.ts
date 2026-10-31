import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";
import ContributorRow from "../ContributorRow.svelte";

const contributor = {
    login: "someone",
    avatar_url: "https://example.test/a.png",
    html_url: "https://github.com/someone",
    contributions: 12,
};

describe("ContributorRow", () => {
    it("links to the profile of the contributor, in a new tab", () => {
        render(ContributorRow, { props: { contributor } });
        const link = screen.getByRole("link");
        expect(link).toHaveAttribute("href", "https://github.com/someone");
        expect(link).toHaveAttribute("target", "_blank");
        expect(link).toHaveAttribute("rel", "noopener noreferrer");
    });

    it("shows the name and the number of commits", () => {
        render(ContributorRow, { props: { contributor } });
        expect(screen.getByText("@someone")).toBeInTheDocument();
        expect(screen.getByText("12 commits")).toBeInTheDocument();
    });

    it("marks the maintainer", () => {
        render(ContributorRow, { props: { contributor, isMaintainer: true } });
        expect(screen.getByText("maintainer")).toBeInTheDocument();
    });

    it("does not mark anyone else as the maintainer", () => {
        render(ContributorRow, { props: { contributor } });
        expect(screen.queryByText("maintainer")).toBeNull();
    });
});

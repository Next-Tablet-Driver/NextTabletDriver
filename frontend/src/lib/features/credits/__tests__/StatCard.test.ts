import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/svelte";
import { Star } from "lucide-svelte";
import StatCard from "../StatCard.svelte";

describe("StatCard", () => {
    it("shows the value and its name", () => {
        render(StatCard, { props: { icon: Star, value: 128, name: "Stars" } });
        expect(screen.getByText("128")).toBeInTheDocument();
        expect(screen.getByText("Stars")).toBeInTheDocument();
    });

    it("renders the given icon", () => {
        const { container } = render(StatCard, { props: { icon: Star, value: "1.2k", name: "Downloads" } });
        expect(container.querySelector("svg")).not.toBeNull();
    });
});

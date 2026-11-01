import { render, screen } from "@testing-library/react";
import App from "@/App";

describe("App", () => {
  it("renders the application name", () => {
    render(<App />);
    expect(screen.getByRole("button", { name: "NextTabletDriver" })).toBeInTheDocument();
  });
});

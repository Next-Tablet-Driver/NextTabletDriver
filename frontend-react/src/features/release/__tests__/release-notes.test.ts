import { describe, expect, it } from "vitest";
import { formatDate, parseBody, parseTextWithLinks } from "@/features/release/release-notes";

describe("parseBody", () => {
  it("is empty for no body", () => {
    expect(parseBody(null)).toEqual({ items: [], contributors: [], fullChangelog: null });
    expect(parseBody("")).toEqual({ items: [], contributors: [], fullChangelog: null });
  });

  it("keeps list items, drops headings and short lines, and finds the changelog link", () => {
    const body = [
      "## What's Changed",
      "* Fix the thing by @alice in https://github.com/o/r/pull/12",
      "- Add another thing by @bob",
      "ok",
      "",
      "**Full Changelog**: https://github.com/o/r/compare/v1...v2",
    ].join("\n");
    expect(parseBody(body)).toEqual({
      items: ["Fix the thing by @alice", "Add another thing by @bob"],
      contributors: ["alice", "bob"],
      fullChangelog: "https://github.com/o/r/compare/v1...v2",
    });
  });

  it("lists a contributor once", () => {
    expect(parseBody("* a by @alice\n* b by @alice").contributors).toEqual(["alice"]);
  });
});

describe("parseTextWithLinks", () => {
  it("splits text, links and mentions", () => {
    expect(parseTextWithLinks("Fixed by @alice, see https://github.com/o/r/pull/7 now")).toEqual([
      { type: "text", content: "Fixed by " },
      { type: "user", content: "alice" },
      { type: "text", content: ", see " },
      { type: "url", content: "https://github.com/o/r/pull/7", display: "#7" },
      { type: "text", content: " now" },
    ]);
  });

  it("names a comparison by its range", () => {
    expect(parseTextWithLinks("https://github.com/o/r/compare/v1...v2")).toEqual([
      { type: "url", content: "https://github.com/o/r/compare/v1...v2", display: "v1...v2" },
    ]);
  });

  it("leaves plain text alone", () => {
    expect(parseTextWithLinks("nothing here")).toEqual([{ type: "text", content: "nothing here" }]);
  });
});

describe("formatDate", () => {
  it("is empty without a date", () => {
    expect(formatDate(null)).toBe("");
    expect(formatDate("")).toBe("");
  });

  it("writes the month in full", () => {
    expect(formatDate("2026-03-05T12:00:00Z")).toBe("March 5, 2026");
  });
});

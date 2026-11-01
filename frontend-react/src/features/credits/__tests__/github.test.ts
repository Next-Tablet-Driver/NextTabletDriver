import { describe, expect, it } from "vitest";
import { orderContributors, totalDownloads, type Contributor } from "@/features/credits/github";

const person = (login: string, contributions: number): Contributor => ({
  login,
  avatar_url: "",
  html_url: "",
  contributions,
});

describe("totalDownloads", () => {
  it("counts installers and ignores the updater's manifests and signatures", () => {
    expect(
      totalDownloads([
        {
          assets: [
            { name: "setup.exe", download_count: 10 },
            { name: "latest.json", download_count: 500 },
            { name: "setup.exe.sig", download_count: 500 },
          ],
        },
        { assets: [{ name: "app.deb", download_count: 5 }] },
      ]),
    ).toBe(15);
  });

  it("is zero without releases", () => {
    expect(totalDownloads([])).toBe(0);
  });
});

describe("orderContributors", () => {
  const list = [
    person("small", 2),
    person("iSweat-exe", 100),
    person("big", 40),
    person("dependabot[bot]", 99),
  ];

  it("puts the maintainer apart, whatever the case of the login", () => {
    expect(orderContributors(list).maintainer?.login).toBe("iSweat-exe");
  });

  it("sorts the others by commits and drops bots", () => {
    expect(orderContributors(list).others.map((c) => c.login)).toEqual(["big", "small"]);
  });

  it("works without the maintainer", () => {
    expect(orderContributors([person("big", 1)]).maintainer).toBeUndefined();
  });
});

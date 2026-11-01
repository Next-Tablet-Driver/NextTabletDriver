const REPOSITORY = "https://api.github.com/repos/Next-Tablet-Driver/NextTabletDriver";
const MAINTAINER = "isweat-exe";

export interface Contributor {
  login: string;
  avatar_url: string;
  html_url: string;
  contributions: number;
}

interface Asset {
  name: string;
  download_count: number;
}

interface GithubRelease {
  assets: Asset[];
}

export interface Credits {
  contributors: Contributor[];
  downloads: number;
  stars: number;
  releases: number;
}

/** Downloads of the installers. The updater's manifests and signatures would count updates twice. */
export function totalDownloads(releases: readonly GithubRelease[]): number {
  return releases.reduce(
    (sum, release) =>
      sum +
      release.assets
        .filter((asset) => !asset.name.endsWith(".json") && !asset.name.endsWith(".sig"))
        .reduce((count, asset) => count + asset.download_count, 0),
    0,
  );
}

/** The maintainer first, then everyone else by number of commits, without bots. */
export function orderContributors(contributors: readonly Contributor[]): {
  maintainer: Contributor | undefined;
  others: Contributor[];
} {
  const isMaintainer = (c: Contributor): boolean => c.login.toLowerCase() === MAINTAINER;
  return {
    maintainer: contributors.find(isMaintainer),
    others: contributors
      .filter((c) => !isMaintainer(c) && !c.login.toLowerCase().includes("bot"))
      .sort((a, b) => b.contributions - a.contributions),
  };
}

/** Reads the repository's numbers; a part that GitHub refuses (rate limit) is left at zero. */
export async function loadCredits(): Promise<Credits> {
  const [contributors, releases, repository] = await Promise.all([
    fetch(`${REPOSITORY}/contributors`),
    fetch(`${REPOSITORY}/releases?per_page=100`),
    fetch(REPOSITORY),
  ]);
  const list = contributors.ok ? ((await contributors.json()) as Contributor[]) : [];
  const published = releases.ok ? ((await releases.json()) as GithubRelease[]) : [];
  const details = repository.ok
    ? ((await repository.json()) as { stargazers_count: number })
    : { stargazers_count: 0 };
  return {
    contributors: list,
    downloads: totalDownloads(published),
    stars: details.stargazers_count,
    releases: published.length,
  };
}

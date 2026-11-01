/** Turns the body of a GitHub release into a short list of changes. */

export interface ParsedBody {
  items: string[];
  contributors: string[];
  fullChangelog: string | null;
}

/** A piece of a change line: plain text, a link, or a GitHub user. */
export type Part =
  | { type: "text"; content: string }
  | { type: "url"; content: string; display: string }
  | { type: "user"; content: string };

const USER = /@([a-zA-Z0-9-]+)/g;

/** The list items of a release body, without headings, and who they mention. */
export function parseBody(body: string | null): ParsedBody {
  const result: ParsedBody = { items: [], contributors: [], fullChangelog: null };
  if (body === null || body === "") {
    return result;
  }
  const add = (text: string): void => {
    const cleaned = text.replace(/ in https:\/\/github\.com\/\S+/g, "");
    result.items.push(cleaned);
    for (const match of cleaned.matchAll(USER)) {
      const user = match[1];
      if (user !== undefined && !result.contributors.includes(user)) {
        result.contributors.push(user);
      }
    }
  };
  for (const raw of body.split("\n")) {
    const line = raw.trim();
    if (line === "") {
      continue;
    }
    if (line.includes("**Full Changelog**:")) {
      result.fullChangelog = line.replace("**Full Changelog**:", "").trim();
    } else if (line.startsWith("#")) {
      continue;
    } else if (line.startsWith("*") || line.startsWith("-")) {
      add(line.slice(1).trim());
    } else if (line.length > 5) {
      add(line);
    }
  }
  return result;
}

function urlPart(url: string): Part {
  let display = url;
  const pull = /\/pull\/(\d+)/.exec(url);
  if (pull?.[1] !== undefined) {
    display = `#${pull[1]}`;
  }
  const compare = /\/compare\/(.*)/.exec(url);
  if (compare?.[1] !== undefined) {
    display = compare[1];
  }
  return { type: "url", content: url, display };
}

/** Splits a line into text, links (`#12` for a pull request) and `@user` mentions. */
export function parseTextWithLinks(text: string): Part[] {
  const parts: Part[] = [];
  let last = 0;
  for (const match of text.matchAll(/(https?:\/\/[^\s]+)|@([a-zA-Z0-9-]+)/g)) {
    if (match.index > last) {
      parts.push({ type: "text", content: text.slice(last, match.index) });
    }
    const [, url, user] = match;
    if (url !== undefined) {
      parts.push(urlPart(url));
    } else if (user !== undefined) {
      parts.push({ type: "user", content: user });
    }
    last = match.index + match[0].length;
  }
  if (last < text.length) {
    parts.push({ type: "text", content: text.slice(last) });
  }
  return parts;
}

export function formatDate(date: string | null): string {
  if (date === null || date === "") {
    return "";
  }
  return new Date(date).toLocaleDateString("en-US", {
    month: "long",
    day: "numeric",
    year: "numeric",
  });
}

import { LinkExternal01 } from "@untitledui/icons";
import { Badge } from "@/components/base/badges/badges";
import { LoadingIndicator } from "@/components/application/loading-indicator/loading-indicator";
import { formatDate, parseBody, parseTextWithLinks } from "@/features/release/release-notes";
import { useCommand } from "@/hooks/useCommand";
import { os } from "@/infrastructure/os";
import { getReleases, type Release } from "@/infrastructure/tauri/commands";
import { Page } from "@/ui/Page";

function Link({ href, children }: { href: string; children: string }) {
  return (
    <a
      href={href}
      className="font-medium text-brand-secondary underline-offset-2 hover:underline"
      onClick={(e) => {
        // The system browser opens it: the webview must not navigate away.
        e.preventDefault();
        void os.openUrl(href);
      }}
    >
      {children}
    </a>
  );
}

function ReleaseCard({ release, isLatest }: { release: Release; isLatest: boolean }) {
  const { items, fullChangelog } = parseBody(release.body);
  return (
    <article className="rounded-xl bg-primary shadow-xs ring-1 ring-secondary">
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-secondary px-5 py-4">
        <div className="flex items-center gap-2">
          <h2 className="text-lg font-semibold text-primary">{release.tag_name}</h2>
          {isLatest && (
            <Badge type="pill-color" size="md" color="success">
              Latest
            </Badge>
          )}
        </div>
        <time className="text-sm text-tertiary" dateTime={release.published_at ?? undefined}>
          {formatDate(release.published_at)}
        </time>
      </header>
      <div className="flex flex-col gap-3 px-5 py-4">
        {items.length === 0 ? (
          <p className="text-sm text-tertiary">No notes for this release.</p>
        ) : (
          <ul className="flex list-disc flex-col gap-2 pl-5 text-sm text-secondary marker:text-quaternary">
            {items.map((item, index) => (
              <li key={index}>
                {parseTextWithLinks(item).map((part, i) => {
                  switch (part.type) {
                    case "url":
                      return (
                        <Link key={i} href={part.content}>
                          {part.display}
                        </Link>
                      );
                    case "user":
                      return (
                        <Link key={i} href={`https://github.com/${part.content}`}>
                          {`@${part.content}`}
                        </Link>
                      );
                    case "text":
                      return <span key={i}>{part.content}</span>;
                  }
                })}
              </li>
            ))}
          </ul>
        )}
        {fullChangelog !== null && (
          <p className="flex items-center gap-1.5 text-sm text-tertiary">
            <LinkExternal01 className="size-4" />
            <Link href={fullChangelog}>Full changelog</Link>
          </p>
        )}
      </div>
    </article>
  );
}

/** What changed in each version of the driver, read from GitHub. */
export function ReleasePage() {
  const { state } = useCommand(getReleases);

  return (
    <Page title="Release notes" description="What is new and improved in the driver.">
      {state.status === "loading" && (
        <div className="flex items-center gap-3 text-sm text-tertiary" role="status">
          <LoadingIndicator type="line-simple" size="sm" />
          Fetching releases from GitHub…
        </div>
      )}
      {state.status === "error" && (
        <p role="alert" className="text-sm text-error-primary">
          Failed to load releases: {state.message}
        </p>
      )}
      {state.status === "ready" && (
        <div className="flex flex-col gap-4">
          {state.data.map((release, index) => (
            <ReleaseCard key={release.tag_name} release={release} isLatest={index === 0} />
          ))}
        </div>
      )}
    </Page>
  );
}

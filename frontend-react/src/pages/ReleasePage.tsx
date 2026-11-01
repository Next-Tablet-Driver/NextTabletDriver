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

function ReleaseNotes({ release, isLatest }: { release: Release; isLatest: boolean }) {
  const { items, fullChangelog } = parseBody(release.body);
  return (
    <article className="flex flex-col gap-3 border-t border-primary pt-5">
      <header className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <h2 className="text-md font-semibold text-primary">
          {release.tag_name}
          {isLatest && <span className="ml-2 text-sm font-medium text-secondary">Latest</span>}
        </h2>
        <time className="text-sm text-tertiary" dateTime={release.published_at ?? undefined}>
          {formatDate(release.published_at)}
        </time>
      </header>
      {items.length === 0 ? (
        <p className="text-sm text-tertiary">No notes for this release.</p>
      ) : (
        <ul className="flex list-disc flex-col gap-1.5 pl-5 text-sm text-secondary marker:text-quaternary">
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
        <p className="text-sm">
          <Link href={fullChangelog}>Full changelog</Link>
        </p>
      )}
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
        <div className="flex flex-col gap-8">
          {state.data.map((release, index) => (
            <ReleaseNotes key={release.tag_name} release={release} isLatest={index === 0} />
          ))}
        </div>
      )}
    </Page>
  );
}

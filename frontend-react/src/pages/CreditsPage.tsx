import { ExternalLink } from "@getpapillon/papicons";
import { Avatar } from "@/components/base/avatar/avatar";
import { LoadingIndicator } from "@/components/application/loading-indicator/loading-indicator";
import { loadCredits, orderContributors, type Contributor } from "@/features/credits/github";
import { useCommand } from "@/hooks/useCommand";
import { Page } from "@/ui/Page";

const number = new Intl.NumberFormat("en-US");

function ContributorRow({
  contributor,
  isMaintainer = false,
}: {
  contributor: Contributor;
  isMaintainer?: boolean;
}) {
  return (
    <a
      href={contributor.html_url}
      target="_blank"
      rel="noopener noreferrer"
      className="flex items-center justify-between gap-3 py-2.5 outline-focus-ring transition-colors duration-150 hover:bg-primary_hover focus-visible:outline-2 focus-visible:-outline-offset-2"
    >
      <span className="flex min-w-0 items-center gap-3">
        <Avatar src={contributor.avatar_url} alt="" size="md" />
        <span className="truncate text-sm font-medium text-primary">@{contributor.login}</span>
        {isMaintainer && <span className="text-sm text-tertiary">Maintainer</span>}
      </span>
      <span className="flex shrink-0 items-center gap-2 text-sm text-tertiary tabular-nums">
        {contributor.contributions} commits
        <ExternalLink className="size-4" aria-hidden="true" />
      </span>
    </a>
  );
}

/** The people and the numbers behind the project. */
export function CreditsPage() {
  const { state } = useCommand(loadCredits);

  return (
    <Page
      title="Credits"
      description="A special thanks to everyone who contributed to NextTabletDriver."
    >
      {state.status === "loading" && (
        <div className="flex items-center gap-3 text-sm text-tertiary" role="status">
          <LoadingIndicator type="line-simple" size="sm" />
          Fetching stats and contributors…
        </div>
      )}
      {state.status === "error" && (
        <p role="alert" className="text-sm text-error-primary">
          Could not reach GitHub: {state.message}
        </p>
      )}
      {state.status === "ready" && (
        <>
          <dl className="flex flex-wrap gap-x-10 gap-y-3 border-t border-primary pt-5">
            {[
              ["Downloads", state.data.downloads],
              ["Stars", state.data.stars],
              ["Releases", state.data.releases],
            ].map(([label, value]) => (
              <div key={label} className="flex items-baseline gap-2">
                <dd className="text-lg font-semibold text-primary tabular-nums">
                  {number.format(Number(value))}
                </dd>
                <dt className="text-sm text-tertiary">{label}</dt>
              </div>
            ))}
          </dl>
          <ul className="flex flex-col divide-y divide-secondary border-t border-primary">
            {(() => {
              const { maintainer, others } = orderContributors(state.data.contributors);
              return [
                ...(maintainer === undefined
                  ? []
                  : [{ contributor: maintainer, isMaintainer: true }]),
                ...others.map((contributor) => ({ contributor, isMaintainer: false })),
              ].map(({ contributor, isMaintainer }) => (
                <li key={contributor.login}>
                  <ContributorRow contributor={contributor} isMaintainer={isMaintainer} />
                </li>
              ));
            })()}
          </ul>
        </>
      )}
    </Page>
  );
}

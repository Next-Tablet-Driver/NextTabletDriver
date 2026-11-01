import type { FC } from "react";
import { Download01, LinkExternal01, Star01, Tag01 } from "@untitledui/icons";
import { Avatar } from "@/components/base/avatar/avatar";
import { Badge } from "@/components/base/badges/badges";
import { LoadingIndicator } from "@/components/application/loading-indicator/loading-indicator";
import { FeaturedIcon } from "@/components/foundations/featured-icon/featured-icon";
import { loadCredits, orderContributors, type Contributor } from "@/features/credits/github";
import { useCommand } from "@/hooks/useCommand";
import { Page } from "@/ui/Page";

const number = new Intl.NumberFormat("en-US");

function Stat({
  icon,
  label,
  value,
}: {
  icon: FC<{ className?: string }>;
  label: string;
  value: number;
}) {
  return (
    <div className="flex items-center gap-4 rounded-xl bg-primary p-5 shadow-xs ring-1 ring-secondary">
      <FeaturedIcon icon={icon} color="brand" theme="light" size="lg" />
      <div className="flex flex-col">
        <span className="text-display-xs font-semibold text-primary">{number.format(value)}</span>
        <span className="text-sm text-tertiary">{label}</span>
      </div>
    </div>
  );
}

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
      className="flex items-center justify-between gap-3 rounded-xl bg-primary px-4 py-3 shadow-xs ring-1 ring-secondary outline-focus-ring transition hover:bg-primary_hover focus-visible:outline-2 focus-visible:outline-offset-2"
    >
      <span className="flex min-w-0 items-center gap-3">
        <Avatar src={contributor.avatar_url} alt="" size="md" />
        <span className="truncate text-sm font-medium text-primary">@{contributor.login}</span>
        {isMaintainer && (
          <Badge type="pill-color" size="sm" color="brand">
            Maintainer
          </Badge>
        )}
      </span>
      <span className="flex shrink-0 items-center gap-2 text-sm text-tertiary">
        {contributor.contributions} commits
        <LinkExternal01 className="size-4" aria-hidden="true" />
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
          <div className="grid gap-4 sm:grid-cols-3">
            <Stat icon={Download01} label="Downloads" value={state.data.downloads} />
            <Stat icon={Star01} label="Stars" value={state.data.stars} />
            <Stat icon={Tag01} label="Releases" value={state.data.releases} />
          </div>
          <ul className="flex flex-col gap-2">
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

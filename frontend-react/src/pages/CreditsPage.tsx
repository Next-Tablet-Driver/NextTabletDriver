import { Download, Star, Tag } from "lucide-react";
import { ContributorRow } from "@/features/credits/ContributorRow";
import { StatCard } from "@/features/credits/StatCard";
import { loadCredits, orderContributors } from "@/features/credits/github";
import { useCommand } from "@/hooks/useCommand";
import styles from "@/pages/CreditsPage.module.css";

const number = new Intl.NumberFormat("en-US");

/** The people and the numbers behind the project. */
export function CreditsPage() {
  const { state } = useCommand(loadCredits);

  return (
    <div className={styles["credits-container"]}>
      <div className={styles.header}>
        <h1 className={styles["page-title"]}>Credits</h1>
        <p className={styles.subtitle}>
          A special thanks to everyone who contributed to NextTabletDriver.
        </p>
      </div>
      {state.status === "loading" && (
        <div className={styles["loading-state"]}>Fetching stats and contributors...</div>
      )}
      {state.status === "ready" && (
        <>
          <div className={styles["stats-cards"]}>
            <StatCard
              icon={Download}
              name="Downloads"
              value={number.format(state.data.downloads)}
            />
            <StatCard icon={Star} name="Stars" value={number.format(state.data.stars)} />
            <StatCard icon={Tag} name="Releases" value={number.format(state.data.releases)} />
          </div>
          <div className={styles["contributors-list"]}>
            {(() => {
              const { maintainer, others } = orderContributors(state.data.contributors);
              return (
                <>
                  {maintainer !== undefined && (
                    <ContributorRow contributor={maintainer} isMaintainer />
                  )}
                  {others.map((contributor) => (
                    <ContributorRow key={contributor.login} contributor={contributor} />
                  ))}
                </>
              );
            })()}
          </div>
        </>
      )}
    </div>
  );
}

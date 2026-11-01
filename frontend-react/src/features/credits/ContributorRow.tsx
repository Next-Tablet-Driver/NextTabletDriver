import { ExternalLink, Link } from "lucide-react";
import type { Contributor } from "@/features/credits/github";
import { classes } from "@/ui/classes";
import styles from "@/features/credits/ContributorRow.module.css";

export function ContributorRow({
  contributor,
  isMaintainer = false,
}: {
  contributor: Contributor;
  isMaintainer?: boolean;
}) {
  return (
    <a
      className={classes(styles["contributor-row"], isMaintainer && styles["maintainer-row"])}
      href={contributor.html_url}
      rel="noopener noreferrer"
      target="_blank"
    >
      <div className={styles["c-left"]}>
        <img
          className={styles.avatar}
          alt={contributor.login}
          src={contributor.avatar_url === "" ? undefined : contributor.avatar_url}
        />
        <div className={styles["c-info"]}>
          <span className={styles["c-name"]}>
            @{contributor.login}
            {isMaintainer && (
              <span className={classes(styles.tag, styles["tag-maintainer"])}>Maintainer</span>
            )}
          </span>
        </div>
      </div>
      <div className={styles["c-right"]}>
        <span className={styles["c-commits"]}>{contributor.contributions} commits</span>
        <span className={styles["link-icon"]}>
          {isMaintainer ? <Link size={18} /> : <ExternalLink size={16} />}
        </span>
      </div>
    </a>
  );
}

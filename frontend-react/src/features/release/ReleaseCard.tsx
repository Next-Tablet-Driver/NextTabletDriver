import type { ReactNode } from "react";
import { ArrowUpRight } from "lucide-react";
import { formatDate, parseBody, parseTextWithLinks } from "@/features/release/release-notes";
import { os } from "@/infrastructure/os";
import type { Release } from "@/infrastructure/tauri/commands";
import { classes } from "@/ui/classes";
import styles from "@/features/release/ReleaseCard.module.css";

const REPOSITORY = "https://github.com/Next-Tablet-Driver/NextTabletDriver";

/** A link that opens in the system browser: the webview must not navigate away. */
function ExternalAnchor({
  href,
  className,
  label,
  children,
}: {
  href: string;
  className?: string;
  label?: string;
  children: ReactNode;
}) {
  return (
    <a
      className={className}
      aria-label={label}
      href={href}
      onClick={(e) => {
        e.preventDefault();
        void os.openUrl(href);
      }}
    >
      {children}
    </a>
  );
}

export function ReleaseCard({
  release,
  isLatest = false,
}: {
  release: Release;
  isLatest?: boolean;
}) {
  const parsed = parseBody(release.body);
  const releaseUrl = `${REPOSITORY}/releases/tag/${release.tag_name}`;

  return (
    <div className={classes(styles["release-card"], isLatest && styles["is-latest"])}>
      <div className={styles["card-header"]}>
        <div className={styles["title-group"]}>
          <h2>{release.tag_name}</h2>
          {isLatest && <span className={classes(styles.badge, styles.latest)}>Latest</span>}
        </div>
        <span className={styles.date}>{formatDate(release.published_at)}</span>
      </div>

      {parsed.items.length > 0 && (
        <div className={styles.changelog}>
          {parsed.items.map((item, itemIndex) => (
            <div key={itemIndex} className={styles["changelog-item"]}>
              <span className={styles.bullet} />
              <span className={styles["item-text"]}>
                {parseTextWithLinks(item).map((part, index) => {
                  switch (part.type) {
                    case "url":
                      return (
                        <ExternalAnchor
                          key={index}
                          className={styles["link-url"]}
                          href={part.content}
                        >
                          {part.display}
                        </ExternalAnchor>
                      );
                    case "user":
                      return (
                        <ExternalAnchor
                          key={index}
                          className={styles["user-mention"]}
                          href={`https://github.com/${part.content}`}
                        >
                          @{part.content}
                        </ExternalAnchor>
                      );
                    case "text":
                      return <span key={index}>{part.content}</span>;
                  }
                })}
              </span>
            </div>
          ))}
        </div>
      )}

      <div className={styles["card-footer"]}>
        {parsed.contributors.length > 0 ? (
          <div className={styles.contributors}>
            {parsed.contributors.slice(0, 8).map((user) => (
              <ExternalAnchor key={user} label={user} href={`https://github.com/${user}`}>
                <img
                  className={styles.avatar}
                  alt={user}
                  src={`https://github.com/${user}.png?size=40`}
                  title={`@${user}`}
                />
              </ExternalAnchor>
            ))}
            {parsed.contributors.length > 8 && (
              <div className={styles["avatar-more"]}>+{parsed.contributors.length - 8}</div>
            )}
          </div>
        ) : (
          <div />
        )}
        <div className={styles.actions}>
          {parsed.fullChangelog !== null && (
            <ExternalAnchor className={styles["btn-link"]} href={parsed.fullChangelog}>
              <span>View Commits</span>
            </ExternalAnchor>
          )}
          <ExternalAnchor className={styles["btn-link"]} href={releaseUrl}>
            <span>GitHub Release</span>
            <ArrowUpRight size={14} />
          </ExternalAnchor>
        </div>
      </div>
    </div>
  );
}

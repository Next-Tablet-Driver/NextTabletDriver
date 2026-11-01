import { PAGES, hrefOf, type PageId } from "@/app/pages";
import { classes } from "@/ui/classes";
import styles from "@/shell/TabBar.module.css";

/** The pages of the window as a row of buttons; each is a URL fragment, so a reload keeps it. */
export function TabBar({ active }: { active: PageId }) {
  return (
    <div className={styles.tabs}>
      {PAGES.map((page) => (
        <button
          key={page.id}
          className={classes(styles.tab, active === page.id && styles.active)}
          onClick={() => {
            window.location.hash = hrefOf(page.id);
          }}
        >
          {page.label}
        </button>
      ))}
    </div>
  );
}

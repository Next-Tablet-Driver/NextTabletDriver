import type { PageId } from "@/app/pages";
import { PAGES } from "@/app/pages";

/** The page the navigation selected. Each page is added by its own sub-task. */
export function PageView({ page }: { page: PageId }) {
  const label = PAGES.find((p) => p.id === page)?.label ?? page;
  return (
    <section className="p-6">
      <h1 className="text-display-xs font-semibold text-primary">{label}</h1>
    </section>
  );
}

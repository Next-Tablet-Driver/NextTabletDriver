import { PAGES, hrefOf, type PageId } from "@/app/pages";
import { NavList } from "@/components/application/app-navigation/base-components/nav-list";

/** The navigation between the pages. Each entry is a link to a URL fragment. */
export function Sidebar({ current }: { current: PageId }) {
  const items = PAGES.map((page) => ({
    label: page.label,
    href: hrefOf(page.id),
    icon: page.icon,
  }));

  return (
    <nav
      aria-label="Pages"
      className="hidden w-60 shrink-0 overflow-y-auto border-r border-secondary bg-primary md:block"
    >
      <NavList activeUrl={hrefOf(current)} items={items} />
    </nav>
  );
}

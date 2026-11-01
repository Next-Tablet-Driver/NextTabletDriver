import { PAGES, hrefOf, type PageId } from "@/app/pages";
import { NavList } from "@/components/application/app-navigation/base-components/nav-list";
import { Select } from "@/components/base/select/select";

/**
 * The navigation between the pages: a sidebar of links to URL fragments, and, in a narrow window
 * where the sidebar does not fit, a menu with the same pages.
 */
export function Sidebar({ current }: { current: PageId }) {
  const items = PAGES.map((page) => ({
    label: page.label,
    href: hrefOf(page.id),
    icon: page.icon,
  }));

  return (
    <>
      <nav
        aria-label="Pages"
        className="hidden w-60 shrink-0 overflow-y-auto border-r border-secondary bg-primary md:block"
      >
        <NavList activeUrl={hrefOf(current)} items={items} />
      </nav>
      <div className="shrink-0 border-b border-secondary bg-primary p-3 md:hidden">
        <Select
          aria-label="Page"
          size="sm"
          items={PAGES.map((page) => ({ id: page.id, label: page.label, icon: page.icon }))}
          value={current}
          onChange={(key) => {
            window.location.hash = hrefOf(String(key) as PageId);
          }}
        >
          {(item) => <Select.Item {...item} />}
        </Select>
      </div>
    </>
  );
}

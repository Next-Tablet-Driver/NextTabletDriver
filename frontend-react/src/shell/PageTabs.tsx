import type { ReactNode } from "react";
import { PAGES, hrefOf, type PageId } from "@/app/pages";
import { Tab, TabList, TabPanel, Tabs } from "@/components/application/tabs/tabs";

/**
 * The pages of the window as tabs under the title bar. Each tab is a URL fragment, so a reload
 * keeps the page. In a narrow window the tabs scroll sideways rather than wrap.
 */
export function PageTabs({ current, children }: { current: PageId; children: ReactNode }) {
  return (
    <Tabs
      selectedKey={current}
      onSelectionChange={(key) => {
        window.location.hash = hrefOf(String(key) as PageId);
      }}
      className="flex min-h-0 flex-1 flex-col"
    >
      <div className="shrink-0 overflow-x-auto bg-primary px-4 pt-1">
        <TabList type="underline" size="md" aria-label="Pages" className="min-w-max">
          {PAGES.map((page) => (
            <Tab key={page.id} id={page.id} label={page.label} icon={page.icon} />
          ))}
        </TabList>
      </div>
      <TabPanel key={current} id={current} className="min-h-0 flex-1 overflow-y-auto">
        {children}
      </TabPanel>
    </Tabs>
  );
}

import type { PageId } from "@/app/pages";
import { PAGES } from "@/app/pages";
import { OutputPage } from "@/pages/OutputPage";
import { PenSettingsPage } from "@/pages/PenSettingsPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { Page } from "@/ui/Page";

/** The page the navigation selected. */
export function PageView({ page }: { page: PageId }) {
  switch (page) {
    case "output":
      return <OutputPage />;
    case "penSettings":
      return <PenSettingsPage />;
    case "settings":
      return <SettingsPage />;
    default:
      return <Page title={PAGES.find((p) => p.id === page)?.label ?? page}>{null}</Page>;
  }
}

import type { PageId } from "@/app/pages";
import { CreditsPage } from "@/pages/CreditsPage";
import { ConsolePage } from "@/pages/ConsolePage";
import { FiltersPage } from "@/pages/FiltersPage";
import { OutputPage } from "@/pages/OutputPage";
import { PenSettingsPage } from "@/pages/PenSettingsPage";
import { ReleasePage } from "@/pages/ReleasePage";
import { SettingsPage } from "@/pages/SettingsPage";

/** The page the navigation selected. */
export function PageView({ page }: { page: PageId }) {
  switch (page) {
    case "output":
      return <OutputPage />;
    case "console":
      return <ConsolePage />;
    case "filters":
      return <FiltersPage />;
    case "penSettings":
      return <PenSettingsPage />;
    case "release":
      return <ReleasePage />;
    case "credits":
      return <CreditsPage />;
    case "settings":
      return <SettingsPage />;
  }
}

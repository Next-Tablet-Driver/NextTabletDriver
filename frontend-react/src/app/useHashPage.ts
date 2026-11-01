import { useSyncExternalStore } from "react";
import { pageFromHash, type PageId } from "@/app/pages";

function subscribe(listener: () => void): () => void {
  window.addEventListener("hashchange", listener);
  return () => {
    window.removeEventListener("hashchange", listener);
  };
}

/** The page the URL fragment points to. The fragment keeps the page across a reload. */
export function useHashPage(): PageId {
  const hash = useSyncExternalStore(subscribe, () => window.location.hash);
  return pageFromHash(hash);
}

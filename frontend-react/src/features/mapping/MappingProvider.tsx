import { createContext, useContext, useEffect, useMemo, type ReactNode } from "react";
import { MappingStore, type MappingSnapshot } from "@/features/mapping/mapping.store";
import { useStore } from "@/lib/store";

const MappingContext = createContext<MappingStore | null>(null);

/** Owns the configuration being edited and loads it once. */
export function MappingProvider({
  children,
  store,
}: {
  children: ReactNode;
  /** A store that is already loaded, for tests. */
  store?: MappingStore;
}) {
  const owned = useMemo(() => store ?? new MappingStore(), [store]);
  useEffect(() => {
    if (store === undefined) {
      void owned.load();
    }
  }, [owned, store]);
  return <MappingContext value={owned}>{children}</MappingContext>;
}

/** The store itself: for the actions (`update`, `save`, `load`...). */
export function useMappingStore(): MappingStore {
  const store = useContext(MappingContext);
  if (store === null) {
    throw new Error("useMappingStore must be used inside a MappingProvider");
  }
  return store;
}

/** The configuration, whether it is unsaved, and whether it can be undone; re-renders on change. */
export function useMappingSnapshot(): MappingSnapshot {
  return useStore(useMappingStore().state);
}

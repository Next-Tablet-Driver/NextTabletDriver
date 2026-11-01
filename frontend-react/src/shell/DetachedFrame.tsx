import type { ReactNode } from "react";
import { Cross, Minus } from "@getpapillon/papicons";
import { Button } from "@/components/base/buttons/button";
import { windowControls } from "@/infrastructure/tauri/window";

/** The frame of a second window (debugger, input lag): its own title bar, then the content. */
export function DetachedFrame({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-secondary">
      <header
        data-tauri-drag-region
        className="flex h-11 shrink-0 items-center justify-between bg-primary pl-4"
      >
        <span data-tauri-drag-region className="truncate text-sm font-semibold text-primary">
          {title}
        </span>
        <div className="flex items-center pr-1">
          <Button
            color="tertiary"
            size="sm"
            aria-label="Minimize"
            iconLeading={Minus}
            onPress={() => void windowControls.minimize()}
          />
          <Button
            color="tertiary"
            size="sm"
            aria-label="Maximize"
            iconLeading={
              <span aria-hidden="true" className="size-3 rounded-[2px] border-2 border-current" />
            }
            onPress={() => void windowControls.toggleMaximize()}
          />
          <Button
            color="tertiary-destructive"
            size="sm"
            aria-label="Close"
            iconLeading={Cross}
            onPress={() => void windowControls.close()}
          />
        </div>
      </header>
      <main className="min-h-0 flex-1 overflow-y-auto">{children}</main>
    </div>
  );
}

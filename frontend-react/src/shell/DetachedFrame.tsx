import type { ReactNode } from "react";
import { Minus, Square, XClose } from "@untitledui/icons";
import { Button } from "@/components/base/buttons/button";
import { windowControls } from "@/infrastructure/tauri/window";

/** The frame of a second window (debugger, input lag): its own title bar, then the content. */
export function DetachedFrame({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-secondary">
      <header
        data-tauri-drag-region
        className="flex h-12 shrink-0 items-center justify-between border-b border-secondary bg-primary pl-4"
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
            iconLeading={Square}
            onPress={() => void windowControls.toggleMaximize()}
          />
          <Button
            color="tertiary-destructive"
            size="sm"
            aria-label="Close"
            iconLeading={XClose}
            onPress={() => void windowControls.close()}
          />
        </div>
      </header>
      <main className="min-h-0 flex-1 overflow-y-auto">{children}</main>
    </div>
  );
}

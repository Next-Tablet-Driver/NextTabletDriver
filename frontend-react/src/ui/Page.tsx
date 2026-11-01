import type { ReactNode } from "react";

/** The frame of a page: a title, what the page is for, and the content under them. */
export function Page({
  title,
  description,
  actions,
  children,
}: {
  title: string;
  description?: string;
  /** Buttons next to the title. */
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-4 py-6 md:px-8 md:py-8">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <h1 className="text-display-xs font-semibold text-primary">{title}</h1>
          {description !== undefined && <p className="text-md text-tertiary">{description}</p>}
        </div>
        {actions}
      </header>
      {children}
    </div>
  );
}

/** A titled group of controls, drawn as a card. */
export function Section({
  title,
  description,
  actions,
  children,
}: {
  title: string;
  description?: string;
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="rounded-xl bg-primary shadow-xs ring-1 ring-secondary">
      <div className="flex flex-wrap items-start justify-between gap-3 border-b border-secondary px-5 py-4">
        <div className="flex flex-col gap-0.5">
          <h2 className="text-lg font-semibold text-primary">{title}</h2>
          {description !== undefined && <p className="text-sm text-tertiary">{description}</p>}
        </div>
        {actions}
      </div>
      <div className="flex flex-col gap-5 px-5 py-5">{children}</div>
    </section>
  );
}

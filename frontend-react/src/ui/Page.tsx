import type { ReactNode } from "react";

/** The frame of a page: its title, what it is for, and the content under them. */
export function Page({
  title,
  description,
  actions,
  children,
}: {
  title: string;
  description?: string;
  /** Buttons on the same line as the title. */
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-8 px-5 py-6 md:px-8">
      <header className="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div className="flex flex-col gap-1">
          <h1 className="text-xl font-semibold text-primary">{title}</h1>
          {description !== undefined && <p className="text-sm text-tertiary">{description}</p>}
        </div>
        {actions}
      </header>
      {children}
    </div>
  );
}

/**
 * A group of settings under a heading. Not a card: the groups are told apart by a rule and by
 * space, so a page reads as one list.
 */
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
    <section className="flex flex-col gap-3 border-t border-primary pt-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex flex-col gap-0.5">
          <h2 className="text-md font-semibold text-primary">{title}</h2>
          {description !== undefined && <p className="text-sm text-tertiary">{description}</p>}
        </div>
        {actions}
      </div>
      <div className="flex flex-col">{children}</div>
    </section>
  );
}

/**
 * One setting: its name and what it does on the left, its control on the right. The two stack
 * when the window is narrow.
 */
export function Row({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children?: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-2 border-b border-secondary py-3 last:border-b-0 sm:flex-row sm:items-center sm:justify-between sm:gap-6">
      <div className="flex min-w-0 flex-col sm:max-w-[55%]">
        <span className="text-sm font-medium text-primary">{label}</span>
        {hint !== undefined && <span className="text-sm text-tertiary">{hint}</span>}
      </div>
      <div className="flex shrink-0 flex-wrap items-center gap-2 sm:justify-end">{children}</div>
    </div>
  );
}

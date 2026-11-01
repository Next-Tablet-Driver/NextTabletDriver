import type { CSSProperties, ReactNode, Ref } from "react";

const STAGE_HEIGHT_PX = 240;

/**
 * The drawing surface of an area editor: a box of the proportions of the thing it shows (the
 * desktop, the tablet), as large as the card allows. Children are placed in percentages of it.
 */
export function AreaStage({
  ratio,
  label,
  stageRef,
  children,
}: {
  /** Width over height of what is drawn. */
  ratio: number;
  label: string;
  stageRef?: Ref<HTMLDivElement>;
  children: ReactNode;
}) {
  const style: CSSProperties = {
    aspectRatio: String(ratio),
    width: `min(100%, ${String(Math.round(STAGE_HEIGHT_PX * ratio))}px)`,
  };
  return (
    <div
      className="flex items-center justify-center rounded-lg bg-secondary p-3 ring-1 ring-secondary ring-inset"
      style={{ height: STAGE_HEIGHT_PX + 24 }}
    >
      <div
        ref={stageRef}
        role="group"
        aria-label={label}
        className="relative overflow-hidden rounded-md bg-primary ring-1 ring-primary ring-inset"
        style={style}
      >
        {children}
      </div>
    </div>
  );
}

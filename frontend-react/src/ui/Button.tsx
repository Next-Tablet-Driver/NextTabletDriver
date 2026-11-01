import type { ButtonHTMLAttributes } from "react";
import { classes } from "@/ui/classes";
import styles from "@/ui/Button.module.css";

export type ButtonVariant =
  | "default"
  | "primary"
  | "secondary"
  | "tertiary"
  | "ghost"
  | "danger"
  | "danger-ghost"
  | "segment";

/** The `btn` class is also global: other components style the buttons they contain through it. */
export function Button({
  variant = "default",
  className = "",
  ...rest
}: { variant?: ButtonVariant; className?: string } & ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button className={classes(styles.btn, styles[variant], "btn", className)} {...rest} />;
}

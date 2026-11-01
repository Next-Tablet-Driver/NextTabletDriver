import type { KeyboardEvent, MouseEvent, ReactNode } from "react";
import { ChevronRight } from "lucide-react";
import { classes } from "@/ui/classes";
import styles from "@/ui/DropdownItem.module.css";

/** One entry of a menu; `rightContent` replaces the arrow of an entry that opens a submenu. */
export function DropdownItem({
  children,
  rightContent,
  disabled = false,
  hasSubmenu = false,
  onClick,
  onMouseEnter,
  onMouseLeave,
}: {
  children?: ReactNode;
  rightContent?: ReactNode;
  disabled?: boolean;
  hasSubmenu?: boolean;
  onClick?: (event: MouseEvent | KeyboardEvent) => void;
  onMouseEnter?: (event: MouseEvent) => void;
  onMouseLeave?: (event: MouseEvent) => void;
}) {
  return (
    <div
      className={classes(
        styles["dropdown-item"],
        "dropdown-item",
        disabled && styles.disabled,
        hasSubmenu && styles["has-submenu"],
      )}
      role="menuitem"
      tabIndex={disabled ? -1 : 0}
      onClick={(event) => {
        if (disabled) {
          event.stopPropagation();
          return;
        }
        onClick?.(event);
      }}
      onKeyDown={(event) => {
        if ((event.key === "Enter" || event.key === " ") && !disabled) {
          event.preventDefault();
          onClick?.(event);
        }
      }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      <span className={styles.content}>{children}</span>
      {rightContent !== undefined ? (
        <span className={styles.right}>{rightContent}</span>
      ) : (
        hasSubmenu && (
          <span className={styles.arrow}>
            <ChevronRight size={16} />
          </span>
        )
      )}
    </div>
  );
}

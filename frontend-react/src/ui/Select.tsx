import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import { classes } from "@/ui/classes";
import styles from "@/ui/Select.module.css";

export interface SelectOption<V = string | number> {
  value: V;
  label: string;
  disabled?: boolean;
}

/** A dropdown list. `options` are values or `{ value, label, disabled }`. */
export function Select<T extends string | number>({
  value,
  options = [],
  onChange,
}: {
  value: T;
  options?: (T | SelectOption<T>)[];
  onChange?: (value: T) => void;
}) {
  const normalized = options.map((option): SelectOption<T> =>
    typeof option === "object" ? option : { value: option, label: String(option) },
  );
  const selected = normalized.find((option) => option.value === value);
  const selectedLabel = selected?.label ?? String(value);

  const [isOpen, setIsOpen] = useState(false);
  const [side, setSide] = useState<"bottom" | "top">("bottom");
  const root = useRef<HTMLDivElement>(null);

  const calculateSide = (): "bottom" | "top" => {
    if (root.current === null) {
      return "bottom";
    }
    const rect = root.current.getBoundingClientRect();
    const estimatedHeight = normalized.length * 32 + 10;
    const spaceBelow = window.innerHeight - rect.bottom;
    return spaceBelow < estimatedHeight && rect.top > spaceBelow ? "top" : "bottom";
  };

  useEffect(() => {
    if (!isOpen) {
      return undefined;
    }
    const onOutside = (event: globalThis.MouseEvent): void => {
      if (root.current !== null && !root.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    window.addEventListener("click", onOutside);
    return () => {
      window.removeEventListener("click", onOutside);
    };
  }, [isOpen]);

  const open = (): void => {
    setSide(calculateSide());
    setIsOpen(true);
  };

  const moveSelection = (direction: number): void => {
    const enabled = normalized.filter((option) => option.disabled !== true);
    if (enabled.length === 0) {
      return;
    }
    const current = enabled.findIndex((option) => option.value === value);
    const next =
      enabled[current === -1 ? 0 : (current + direction + enabled.length) % enabled.length];
    if (next !== undefined) {
      onChange?.(next.value);
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>): void => {
    switch (event.key) {
      case "Escape":
        event.preventDefault();
        setIsOpen(false);
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        if (isOpen) {
          setIsOpen(false);
        } else {
          open();
        }
        break;
      case "ArrowDown":
      case "ArrowUp":
        event.preventDefault();
        if (isOpen) {
          moveSelection(event.key === "ArrowDown" ? 1 : -1);
        } else {
          open();
        }
        break;
    }
  };

  return (
    <div ref={root} className={styles.select}>
      <button
        className={classes(styles["select-trigger"], isOpen && styles.open)}
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        type="button"
        onClick={(event: MouseEvent) => {
          event.preventDefault();
          if (isOpen) {
            setIsOpen(false);
          } else {
            open();
          }
        }}
        onKeyDown={onKeyDown}
      >
        <span className={styles["select-value"]}>{selectedLabel}</span>
        <svg
          className={classes(styles["select-chevron"], isOpen && styles.open)}
          aria-hidden="true"
          fill="none"
          viewBox="0 0 16 16"
        >
          <path
            d="M4 6L8 10L12 6"
            stroke="currentColor"
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth="1.5"
          />
        </svg>
      </button>
      {isOpen && (
        <div
          className={styles.dropdown}
          data-side={side}
          role="listbox"
          tabIndex={-1}
          onClick={(event) => {
            event.stopPropagation();
          }}
          onKeyDown={(event) => {
            event.stopPropagation();
          }}
        >
          {normalized.map((option) => (
            <button
              key={option.value}
              className={classes(
                styles.option,
                option.disabled === true && styles.disabled,
                option.value === value && styles.selected,
              )}
              aria-selected={option.value === value}
              disabled={option.disabled}
              role="option"
              type="button"
              onClick={(event) => {
                event.preventDefault();
                if (option.disabled === true) {
                  return;
                }
                onChange?.(option.value);
                setIsOpen(false);
              }}
            >
              <span className={styles["option-label"]}>{option.label}</span>
              {option.value === value && (
                <svg className={styles.check} aria-hidden="true" fill="none" viewBox="0 0 16 16">
                  <path
                    d="M3 8L6.5 11.5L13 4.5"
                    stroke="currentColor"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth="1.8"
                  />
                </svg>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

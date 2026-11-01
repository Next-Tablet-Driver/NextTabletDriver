import type { InputHTMLAttributes } from "react";
import { Tooltip } from "@/ui/Tooltip";
import styles from "@/ui/Checkbox.module.css";

export function Checkbox({
  checked,
  onChange,
  label = "",
  tooltip = "",
  ...rest
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  tooltip?: string;
} & Omit<InputHTMLAttributes<HTMLInputElement>, "checked" | "onChange" | "type">) {
  const field = (
    <label className={styles["checkbox-label"]}>
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => {
          onChange(event.currentTarget.checked);
        }}
        {...rest}
      />
      {label}
    </label>
  );
  return tooltip === "" ? field : <Tooltip text={tooltip}>{field}</Tooltip>;
}

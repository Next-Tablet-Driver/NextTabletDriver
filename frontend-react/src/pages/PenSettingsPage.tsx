import { Pencil } from "lucide-react";
import { useMappingSnapshot } from "@/features/mapping/MappingProvider";
import { BindingCard } from "@/features/pen/BindingCard";
import { ButtonActions } from "@/features/pen/ButtonActions";
import { HardwareToggles } from "@/features/pen/HardwareToggles";
import { PressureCurve } from "@/features/pen/PressureCurve";
import { Thresholds } from "@/features/pen/Thresholds";
import styles from "@/pages/PenSettingsPage.module.css";

export function PenSettingsPage() {
  const { config } = useMappingSnapshot();
  return (
    <div className={styles["page-container"]}>
      <div className={styles["section-header"]}>
        <Pencil size={20} />
        <h2>Pen Settings</h2>
      </div>
      {config !== null && (
        <div className={styles["grid-layout"]}>
          <Thresholds />
          <HardwareToggles />
          <BindingCard type="tip" />
          <BindingCard type="eraser" />
          <PressureCurve />
          <ButtonActions />
        </div>
      )}
    </div>
  );
}

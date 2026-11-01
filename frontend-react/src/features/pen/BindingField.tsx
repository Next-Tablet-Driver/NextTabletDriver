import { useEffect, useRef, useState } from "react";
import { Keyboard } from "lucide-react";
import { buildChord, heldModifiers, keyName, modifierName } from "@/features/pen/binding-capture";
import { normalizeBinding } from "@/infrastructure/tauri/commands";
import { Button } from "@/ui/Button";
import { classes } from "@/ui/classes";
import { Select } from "@/ui/Select";
import styles from "@/features/pen/BindingField.module.css";

/** The bindings that need no keyboard: nothing, or one of the five mouse buttons. */
const PRESETS = ["None", "Left Click", "Right Click", "Middle Click", "Back", "Forward"];
/** What the preset list shows: always its title, because the field next to it shows the binding. */
const PRESET_TITLE = "Presets";

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * One binding: typed, picked from the presets, or recorded from the keyboard. The driver decides
 * whether the text is a binding and gives it back in its plain spelling (`ctrl + z` is `Ctrl+Z`).
 */
export function BindingField({
  id,
  label,
  value,
  onEdit,
}: {
  id: string;
  label: string;
  value: string;
  /** Called with the new binding text, as typed, picked or recorded. */
  onEdit: (text: string) => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const [recording, setRecording] = useState(false);
  /** What has been held so far while recording, shown in the field. */
  const [preview, setPreview] = useState("");
  const seen = useRef<string[]>([]);
  const onEditRef = useRef(onEdit);
  useEffect(() => {
    onEditRef.current = onEdit;
  });

  /** Asks the driver whether the text is a binding, and stores it in its plain spelling. */
  const check = (text: string): void => {
    normalizeBinding(text).then(
      (plain) => {
        setError(null);
        if (plain !== text) {
          onEditRef.current(plain);
        }
      },
      (failure: unknown) => {
        setError(messageOf(failure));
      },
    );
  };

  useEffect(() => {
    if (!recording) {
      return undefined;
    }
    const finish = (text: string | null): void => {
      setRecording(false);
      setPreview("");
      seen.current = [];
      if (text !== null && text !== "") {
        onEditRef.current(text);
        check(text);
      }
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      event.preventDefault();
      event.stopPropagation();
      if (event.repeat) {
        return;
      }
      const modifier = modifierName(event.code);
      if (modifier !== null) {
        if (!seen.current.includes(modifier)) {
          seen.current.push(modifier);
        }
        setPreview(buildChord(seen.current, null));
        return;
      }
      const key = keyName(event.code);
      if (key === null) {
        setError("That key cannot be used in a binding");
        finish(null);
        return;
      }
      finish(buildChord([...seen.current, ...heldModifiers(event)], key));
    };
    // Releasing the modifiers without any other key binds the modifiers alone (`Shift`).
    const onKeyUp = (event: KeyboardEvent): void => {
      event.preventDefault();
      event.stopPropagation();
      if (modifierName(event.code) !== null && seen.current.length > 0) {
        finish(buildChord(seen.current, null));
      }
    };
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
    };
  }, [recording]);

  return (
    <div className={styles["binding-row"]}>
      <label htmlFor={id}>{label}</label>
      <div
        className={classes(
          styles["binding-input"],
          error !== null && styles.invalid,
          recording && styles.recording,
        )}
      >
        <input
          id={id}
          aria-invalid={error !== null}
          autoComplete="off"
          placeholder={recording ? "Press the keys…" : ""}
          readOnly={recording}
          spellCheck={false}
          type="text"
          value={recording ? preview : value}
          onChange={(event) => {
            setError(null);
            onEdit(event.currentTarget.value);
          }}
          onBlur={(event) => {
            if (!recording) {
              check(event.currentTarget.value);
            }
          }}
        />
        <Select
          value={PRESET_TITLE}
          options={PRESETS}
          onChange={(preset) => {
            setError(null);
            onEdit(preset);
          }}
        />
        <Button
          aria-label={`Record ${label}`}
          aria-pressed={recording}
          variant="segment"
          onClick={() => {
            if (recording) {
              setRecording(false);
              setPreview("");
              seen.current = [];
              return;
            }
            setError(null);
            setRecording(true);
            setPreview("");
            seen.current = [];
          }}
        >
          <Keyboard size={14} />
        </Button>
      </div>
      {error !== null ? (
        <p className={styles["binding-error"]} role="alert">
          {error}
        </p>
      ) : (
        recording && (
          <p className={styles["binding-hint"]}>
            Press a key or a combination. Click the keyboard again to cancel.
          </p>
        )
      )}
    </div>
  );
}

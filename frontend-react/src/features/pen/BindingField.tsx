import { useEffect, useRef, useState } from "react";
import { Keyboard01 } from "@untitledui/icons";
import { Button } from "@/components/base/buttons/button";
import { Input } from "@/components/base/input/input";
import { Select } from "@/components/base/select/select";
import { buildChord, heldModifiers, keyName, modifierName } from "@/features/pen/binding-capture";
import { normalizeBinding } from "@/infrastructure/tauri/commands";

/** The bindings that need no keyboard: nothing, or one of the mouse buttons. */
const PRESETS = ["None", "Left Click", "Right Click", "Middle Click", "Back", "Forward"].map(
  (name) => ({ id: name, label: name }),
);

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * One binding: typed, picked from the presets, or recorded from the keyboard. The driver decides
 * whether the text is a binding and gives it back in its plain spelling (`ctrl + z` is `Ctrl+Z`).
 */
export function BindingField({
  label,
  value,
  onEdit,
}: {
  label: string;
  value: string;
  /** Called with the new binding text, as typed, picked or recorded. */
  onEdit: (text: string) => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const [recording, setRecording] = useState(false);
  const [preview, setPreview] = useState("");
  const seen = useRef<string[]>([]);
  const onEditRef = useRef(onEdit);
  useEffect(() => {
    onEditRef.current = onEdit;
  });

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
    <div className="flex flex-col gap-1.5">
      <div className="flex flex-wrap items-end gap-2">
        <div className="min-w-48 flex-1">
          <Input
            label={label}
            value={recording ? preview : value}
            placeholder={recording ? "Press the keys…" : "None"}
            isReadOnly={recording}
            isInvalid={error !== null}
            onChange={(text) => {
              setError(null);
              onEdit(text);
            }}
            onBlur={() => {
              if (!recording) {
                check(value);
              }
            }}
          />
        </div>
        <div className="w-40">
          <Select
            aria-label={`${label}: presets`}
            placeholder="Presets"
            items={PRESETS}
            value={null}
            onChange={(key) => {
              setError(null);
              onEdit(String(key));
            }}
          >
            {(item) => <Select.Item {...item} />}
          </Select>
        </div>
        <Button
          color={recording ? "primary" : "secondary"}
          aria-label={`Record ${label}`}
          aria-pressed={recording}
          iconLeading={Keyboard01}
          onPress={() => {
            setError(null);
            setPreview("");
            seen.current = [];
            setRecording((on) => !on);
          }}
        />
      </div>
      {error !== null ? (
        <p role="alert" className="text-sm text-error-primary">
          {error}
        </p>
      ) : (
        recording && (
          <p className="text-sm text-tertiary">
            Press a key or a combination. Click the keyboard again to cancel.
          </p>
        )
      )}
    </div>
  );
}

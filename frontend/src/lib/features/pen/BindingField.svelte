<script lang="ts">
  import { onDestroy } from "svelte";
  import { Keyboard } from "@lucide/svelte";
  import Button from "../../components/ui/Button.svelte";
  import Select from "../../components/ui/Select.svelte";
  import { normalizeBinding } from "../../infrastructure/tauri/commands";
  import { buildChord, heldModifiers, keyName, modifierName } from "./binding-capture";

  const {
    id,
    label,
    value,
    onedit,
  }: {
    id: string;
    label: string;
    value: string;
    /** Called with the new binding text, as typed, picked or recorded. */
    onedit: (text: string) => void;
  } = $props();

  /** The bindings that need no keyboard: nothing, or one of the five mouse buttons. */
  const PRESETS = ["None", "Left Click", "Right Click", "Middle Click", "Back", "Forward"];

  /** What the preset list shows: always its title, because the field above shows the binding. */
  const PRESET_TITLE = "Presets";
  let presetShown = $state(PRESET_TITLE);

  let error = $state<string | null>(null);
  let recording = $state(false);
  /** What has been held so far while recording, shown in the field. */
  let preview = $state("");
  let seenModifiers: string[] = [];

  /** Asks the driver whether the text is a binding, and stores it in its plain spelling. */
  async function check(text: string): Promise<void> {
    try {
      const plain = await normalizeBinding(text);
      error = null;
      if (plain !== text) onedit(plain);
    } catch (failure) {
      error = failure instanceof Error ? failure.message : String(failure);
    }
  }

  function onInput(event: Event): void {
    error = null;
    onedit((event.currentTarget as HTMLInputElement).value);
  }

  function onChange(event: Event): void {
    void check((event.currentTarget as HTMLInputElement).value);
  }

  function pickPreset(preset: string): void {
    error = null;
    onedit(preset);
    presetShown = PRESET_TITLE;
  }

  function finish(text: string | null): void {
    stopListening();
    recording = false;
    preview = "";
    seenModifiers = [];
    if (text !== null && text !== "") {
      onedit(text);
      void check(text);
    }
  }

  function onKeyDown(event: KeyboardEvent): void {
    event.preventDefault();
    event.stopPropagation();
    if (event.repeat) return;
    const modifier = modifierName(event.code);
    if (modifier !== null) {
      if (!seenModifiers.includes(modifier)) seenModifiers.push(modifier);
      preview = buildChord(seenModifiers, null);
      return;
    }
    const key = keyName(event.code);
    if (key === null) {
      error = "That key cannot be used in a binding";
      finish(null);
      return;
    }
    finish(buildChord([...seenModifiers, ...heldModifiers(event)], key));
  }

  /** Releasing the modifiers without any other key binds the modifiers alone (`Shift`). */
  function onKeyUp(event: KeyboardEvent): void {
    event.preventDefault();
    event.stopPropagation();
    if (modifierName(event.code) !== null && seenModifiers.length > 0) {
      finish(buildChord(seenModifiers, null));
    }
  }

  function stopListening(): void {
    window.removeEventListener("keydown", onKeyDown, true);
    window.removeEventListener("keyup", onKeyUp, true);
  }

  function toggleRecording(): void {
    if (recording) {
      finish(null);
      return;
    }
    error = null;
    recording = true;
    preview = "";
    seenModifiers = [];
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
  }

  onDestroy(stopListening);
</script>

<div class="binding-row">
  <label for={id}>{label}</label>
  <div class="binding-input" class:invalid={error !== null} class:recording>
    <input
      {id}
      aria-invalid={error !== null}
      autocomplete="off"
      onchange={onChange}
      oninput={onInput}
      placeholder={recording ? "Press the keys…" : ""}
      readonly={recording}
      spellcheck="false"
      type="text"
      value={recording ? preview : value}
    />
    <Select onchange={pickPreset} options={PRESETS} bind:value={presetShown} />
    <Button
      aria-label={`Record ${label}`}
      aria-pressed={recording}
      onclick={toggleRecording}
      variant="segment"
    >
      <Keyboard size={14} />
    </Button>
  </div>
  {#if error !== null}
    <p class="binding-error" role="alert">{error}</p>
  {:else if recording}
    <p class="binding-hint">Press a key or a combination. Click the keyboard again to cancel.</p>
  {/if}
</div>

<style>
  .binding-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-bottom: var(--space-4);
  }

  .binding-row label {
    font-size: var(--font-md);
    color: var(--text-active);
    font-weight: var(--font-weight-medium);
  }

  .binding-input {
    display: flex;
    align-items: center;
    background: var(--bg-input);
    border: var(--border-width) solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .binding-input.invalid {
    border-color: var(--danger);
  }

  .binding-input.recording {
    border-color: var(--accent);
  }

  .binding-input input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    color: var(--text-muted);
    padding: var(--space-2) var(--space-3);
    font-size: var(--font-md);
    text-align: left;
    outline: none;
  }

  .binding-input input:focus {
    color: var(--text-active);
  }

  .binding-error {
    margin: 0;
    font-size: var(--font-sm, 0.8em);
    color: var(--danger);
  }

  .binding-hint {
    margin: 0;
    font-size: var(--font-sm, 0.8em);
    color: var(--text-muted);
  }
</style>

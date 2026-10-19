# Button mapping

You can decide what each button on the pen and each key on the tablet does. A button can press a mouse button, a key, or a shortcut such as **Ctrl+Z**.

Everything is on the **Pen Settings** tab:

- **Button Actions** lists the buttons on the pen.
- **Tablet Keys** lists the keys on the tablet itself (Wacom calls them ExpressKeys). It only appears if your tablet has keys.

The app takes the number of rows from the tablet definition. If a row is missing, see [Adding a tablet](adding-a-tablet.md).

## Set a binding

Each row has three ways to set a binding:

1. **Type it** in the text field. When you leave the field, the app checks the text and rewrites it in its standard form (`ctrl + z` becomes `Ctrl+Z`). If the text is not valid, a red message tells you why.
2. **Pick a preset** from the **Presets** list: **None**, **Left Click**, **Right Click**, **Middle Click**, **Back** or **Forward**.
3. **Record it.** Choose the keyboard button next to the field, then press the keys you want. A single key is recorded at once. For a shortcut, hold the modifiers (Ctrl, Shift, Alt, Win) and press the last key. To bind a modifier alone, press and release it. Choose the keyboard button again to cancel.

The binding works at once. It is saved with your profile like every other setting.

## What you can write

| Text | What happens while you hold the button |
| --- | --- |
| `None` | Nothing. This is the default. `Disabled` means the same. |
| `Left Click`, `Right Click`, `Middle Click` | The mouse button is held down. |
| `Back`, `Forward` | The two side buttons of a mouse. Browsers use them to go back and forward. |
| `Space`, `F5`, `Enter`, `A` | The key is held down. |
| `Ctrl+Z`, `Ctrl+Shift+S` | The keys are pressed in this order, and released in the reverse order. |

Rules:

- Names are not case sensitive, and a few spellings are accepted: `Control`, `Del`, `Return`, `PgUp`, `Esc`, `Win`, `Meta`.
- A shortcut has at most four keys, and cannot use the same key twice.
- A mouse button cannot be part of a shortcut. `Ctrl+Right Click` is refused.
- Keys are named by their place on a US keyboard, like scan codes. On a French keyboard, the key written `A` is the one named `Q` in a binding. Use the recorder if you are not sure: it records the place of the key you press.
- Older OpenTabletDriver spellings still load: `Mouse Button Binding: (Button: Left)` and `Key Binding: (Key: Space)`.

The keys you can use are the letters `A` to `Z`, the digits `0` to `9`, `F1` to `F24`, `Esc`, `Tab`, `Enter`, `Space`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, the four arrows, `Ctrl`, `Shift`, `Alt`, `Win`, and the symbols `-` `=` `[` `]` `\` `;` `'` `` ` `` `,` `.` `/`. The number pad and the media keys are not available yet.

## How it behaves

- A binding is held while the physical button is held, so you can drag with **Right Click** or keep a key pressed for a game.
- If you change a binding while its button is held, the old binding is released correctly. No key stays stuck.
- If two bindings share a key (`Ctrl+Z` and `Ctrl+Y` both use `Ctrl`), the system sees `Ctrl` pressed once and released when the last of them is released.
- When the pen leaves the tablet, its buttons are released. When the tablet is unplugged or the engine stops, everything is released, so nothing stays pressed.
- Pen buttons are read while the pen is close to the tablet, with or without touching it.

## Known limits

- A pen button bound to **Left Click** shares the left button with the tip. If you release one while the other is still held, the click ends. Bind the pen buttons to other buttons or keys.
- The **tip** and **eraser** bindings on the same tab are not used yet. The tip always clicks the left mouse button.
- A program that runs as administrator on Windows ignores input from a program that does not. If a key works everywhere except in one program, run NextTabletDriver as administrator too.
- On Linux, the keys are sent through a virtual keyboard created with `/dev/uinput`, like the pen and the mouse. See [Linux setup](linux.md).

## Settings file

The bindings are lists of text in your profile, first button first:

```json
"pen_button_bindings": ["Right Click", "Ctrl+Z"],
"aux_button_bindings": ["Space", "Ctrl+Z", "None", "F5"]
```

A button with no entry does nothing. A text that cannot be read does nothing too, and the **Console** shows a message from `Buttons` when you press that button. Profiles saved before this version have no `aux_button_bindings` and load without problems.

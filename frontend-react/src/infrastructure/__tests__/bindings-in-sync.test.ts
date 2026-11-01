// @vitest-environment node
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Until the switch (R13), the Rust test `apps/desktop/tests/bindings.rs` regenerates the copy in
// the Svelte frontend only. This copy must not drift from it. Delete this test with `frontend/`.
const SVELTE_COPY = fileURLToPath(
  new URL("../../../../frontend/src/lib/infrastructure/tauri/bindings.ts", import.meta.url),
);
const THIS_COPY = fileURLToPath(new URL("../tauri/bindings.ts", import.meta.url));

describe("bindings.ts", () => {
  it.skipIf(!existsSync(SVELTE_COPY))("is the file the Rust code generates", () => {
    expect(readFileSync(THIS_COPY, "utf8")).toBe(readFileSync(SVELTE_COPY, "utf8"));
  });
});

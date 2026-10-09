import { vi, type Mock } from "vitest";
import type { MappingState } from "../features/mapping/mapping-state.svelte";

export interface FakeMapping<C extends object> {
    mapping: MappingState;
    markDirty: Mock;
    /** The very object the components bind to, so a test can read back what they wrote. */
    config: C | null;
}

/**
 * A stand-in for the mapping state: just the loaded config (or `null` before it is loaded) and a
 * spy for `markDirty`. The components write straight into `config`, so assertions should read it
 * back instead of re-reading the DOM value the test has just typed.
 */
export function fakeMapping<C extends object>(config: C | null): FakeMapping<C> {
    const markDirty = vi.fn();
    return { mapping: { config, markDirty } as unknown as MappingState, markDirty, config };
}

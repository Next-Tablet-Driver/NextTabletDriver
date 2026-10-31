<script lang="ts">
    import { onMount } from "svelte";
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import { getTabletStatus } from "../../infrastructure/tauri/commands";
    import BindingField from "./BindingField.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    /** How many buttons the connected tablet says it has; 0 while unknown. */
    let penButtonCount = $state(0);
    let auxButtonCount = $state(0);

    onMount(() => {
        void getTabletStatus()
            .then((status) => {
                penButtonCount = status.pen_button_count;
                auxButtonCount = status.aux_button_count;
            })
            // Outside the app (tests, plain browser) there is no tablet to ask.
            .catch(() => undefined);
    });

    /** A row for every binding already saved, and for every button the tablet really has. */
    function rowCount(saved: number, tablet: number): number {
        return Math.max(saved, tablet);
    }

    const penRows = $derived(rowCount(mapping.config?.pen_button_bindings.length ?? 0, penButtonCount));
    const auxRows = $derived(rowCount(mapping.config?.aux_button_bindings.length ?? 0, auxButtonCount));

    type Kind = "pen" | "aux";

    function bindingsOf(kind: Kind): string[] | null {
        if (mapping.config === null) return null;
        return kind === "pen" ? mapping.config.pen_button_bindings : mapping.config.aux_button_bindings;
    }

    function valueOf(kind: Kind, index: number): string {
        return bindingsOf(kind)?.[index] ?? "None";
    }

    /** Writes one binding, padding the list with `None` up to its position. */
    function edit(kind: Kind, index: number, text: string): void {
        const list = bindingsOf(kind);
        if (list === null) return;
        while (list.length <= index) list.push("None");
        list[index] = text;
        mapping.markDirty();
    }
</script>

<div class="card buttons-card">
    <div class="section-header small">
        <svg fill="none" height="18" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" viewBox="0 0 24 24" width="18"><rect height="20" rx="4" width="12" x="6" y="2"></rect><line x1="12" x2="12.01" y1="18" y2="18"></line></svg>
        <h3>Button Actions</h3>
    </div>

    {#if mapping.config !== null}
        {#each { length: penRows } as _row, index (index)}
            <BindingField
                id={`pen_button_${String(index)}`}
                label={`Pen Button ${String(index + 1)}`}
                onedit={(text: string) => { edit("pen", index, text); }}
                value={valueOf("pen", index)}
            />
        {/each}
    {/if}
</div>

{#if mapping.config !== null && auxRows > 0}
    <div class="card buttons-card">
        <div class="section-header small">
            <svg fill="none" height="18" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" viewBox="0 0 24 24" width="18"><rect height="16" rx="2" width="20" x="2" y="4"></rect><path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M8 12h.01M12 12h.01M16 12h.01M7 16h10"></path></svg>
            <h3>Tablet Keys</h3>
        </div>
        {#each { length: auxRows } as _row, index (index)}
            <BindingField
                id={`aux_button_${String(index)}`}
                label={`Tablet Key ${String(index + 1)}`}
                onedit={(text: string) => { edit("aux", index, text); }}
                value={valueOf("aux", index)}
            />
        {/each}
    </div>
{/if}

<style>
    .card {
        display: flex;
        flex-direction: column;
        gap: var(--space-4);
    }

    .buttons-card {
        grid-column: span 1;
        margin-top: var(--space-6);
    }

    .section-header {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        color: var(--text-active);
    }

    .section-header.small {
        margin-bottom: var(--space-4);
    }

    .section-header.small h3 {
        font-size: var(--font-lg-plus);
        font-weight: var(--font-weight-bold);
        margin: 0;
    }
</style>

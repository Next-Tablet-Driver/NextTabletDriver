<script lang="ts">
    import { onMount } from "svelte";
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import { getTabletStatus } from "../../infrastructure/tauri/commands";
    import Subtitle from "../../components/ui/Subtitle.svelte";
    import BindingField from "./BindingField.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    /** How many buttons the connected tablet says it has; 0 while unknown. */
    let penButtonCount = $state(0);
    let auxButtonCount = $state(0);

    onMount(() => {
        void getTabletStatus()
            .then((status) => {
                penButtonCount = status.pen_button_count ?? 0;
                auxButtonCount = status.aux_button_count ?? 0;
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

<div class="groups">
    {#if mapping.config !== null}
        <div class="group">
            <Subtitle>Button Actions</Subtitle>
            {#each { length: penRows } as _row, index (index)}
                <BindingField
                    id={`pen_button_${String(index)}`}
                    label={`Pen Button ${String(index + 1)}`}
                    onedit={(text: string) => { edit("pen", index, text); }}
                    value={valueOf("pen", index)}
                />
            {/each}
        </div>

        {#if auxRows > 0}
            <div class="group">
                <Subtitle>Tablet Keys</Subtitle>
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
    {/if}
</div>

<style>
    .groups {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
        gap: var(--space-8);
        align-items: start;
    }

    .group {
        display: flex;
        flex-direction: column;
        gap: var(--space-3);
    }
</style>

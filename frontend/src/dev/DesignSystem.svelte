<script lang="ts">
    /**
     * Development-only design system gallery (`/?view=design-system`).
     * Renders every shared primitive and the main tokens in the selected theme, so a change to
     * the design system can be reviewed (and compared with `__ntdSnapshot`) in one place.
     */
    import Button from "../lib/components/ui/Button.svelte";
    import Checkbox from "../lib/components/ui/Checkbox.svelte";
    import Input from "../lib/components/ui/Input.svelte";
    import Select from "../lib/components/ui/Select.svelte";
    import Title from "../lib/components/ui/Title.svelte";
    import Subtitle from "../lib/components/ui/Subtitle.svelte";
    import Text from "../lib/components/ui/Text.svelte";
    import Tooltip from "../lib/components/ui/Tooltip.svelte";
    import { BUILTIN_THEMES, themeStore } from "../lib/theme";

    let selectValue = $state("Option B");
    let numberValue = $state(42);
    let checked = $state(true);
    let unchecked = $state(false);

    const VARIANTS = ["default", "primary", "secondary", "tertiary", "ghost", "danger", "danger-ghost", "segment"] as const;
    const STATUSES = ["info", "warning", "error", "danger", "success", "debug"] as const;

    const COLOR_TOKENS = [
        "--bg-app", "--bg-panel", "--bg-panel-hover", "--bg-input", "--bg-tab-active",
        "--text-main", "--text-muted", "--text-active", "--text-strong", "--text-emphasis", "--text-on-accent", "--text-number",
        "--accent", "--accent-hover", "--border", "--border-hover",
        "--input-bg", "--input-border", "--area-display", "--area-tablet", "--console-text",
    ] as const;

    const SPACE = ["0-5", "1", "2", "3", "4", "6", "8"] as const;
    const RADII = ["sm", "md", "lg", "xl"] as const;
    const FONTS = ["xs", "sm", "base", "md", "lg", "xl", "2xl", "3xl"] as const;
</script>

<main class="gallery">
    <header class="gallery-header">
        <Title>Design system</Title>
        <div class="row" aria-label="Theme" role="group">
            <Button
                onclick={() => { themeStore.select("System"); }}
                variant={themeStore.selection === "System" ? "primary" : "secondary"}
            >system</Button>
            {#each BUILTIN_THEMES as theme (theme.id)}
                <Button
                    onclick={() => { themeStore.select(theme.configValue); }}
                    variant={themeStore.selection === theme.configValue ? "primary" : "secondary"}
                >{theme.id}</Button>
            {/each}
        </div>
    </header>

    <section>
        <Subtitle>Buttons</Subtitle>
        <div class="row">
            {#each VARIANTS as variant (variant)}
                <Button {variant}>{variant}</Button>
            {/each}
        </div>
        <div class="row">
            {#each VARIANTS as variant (variant)}
                <Button disabled {variant}>{variant}</Button>
            {/each}
        </div>
    </section>

    <section>
        <Subtitle>Form controls</Subtitle>
        <div class="row">
            <Input label="Width" unit="mm" bind:value={numberValue} />
            <Input label="Name" type="text" value="Default" width="120px" />
            <Select options={["Option A", "Option B", "Option C"]} bind:value={selectValue} />
            <Checkbox label="Checked" bind:checked />
            <Checkbox label="Unchecked" bind:checked={unchecked} />
            <Checkbox disabled label="Disabled" />
        </div>
    </section>

    <section>
        <Subtitle>Typography</Subtitle>
        <Title>Title</Title>
        <Subtitle>Subtitle</Subtitle>
        <Text>Body text in the main text color.</Text>
        <Tooltip delay={0} text="Tooltip content">
            <Text>Hover for a tooltip</Text>
        </Tooltip>
        <div class="scale">
            {#each FONTS as size (size)}
                <span style:font-size={`var(--font-${size})`}>--font-{size}</span>
            {/each}
        </div>
    </section>

    <section>
        <Subtitle>Status colors</Subtitle>
        <div class="row">
            {#each STATUSES as status (status)}
                <span
                    style:background={`color-mix(in srgb, var(--${status}) 15%, transparent)`}
                    style:border-color={`color-mix(in srgb, var(--${status}) 30%, transparent)`}
                    style:color={`var(--${status})`}
                    class="badge"
                >{status}</span>
            {/each}
        </div>
    </section>

    <section>
        <Subtitle>Space and radius</Subtitle>
        <div class="row">
            {#each SPACE as step (step)}
                <div style:width={`var(--space-${step})`} class="space" title={`--space-${step}`}></div>
            {/each}
        </div>
        <div class="row">
            {#each RADII as radius (radius)}
                <div style:border-radius={`var(--radius-${radius})`} class="radius">{radius}</div>
            {/each}
        </div>
    </section>

    <section>
        <Subtitle>Color tokens</Subtitle>
        <div class="swatches">
            {#each COLOR_TOKENS as token (token)}
                <div class="swatch">
                    <span style:background={`var(${token})`} class="chip"></span>
                    <code>{token}</code>
                </div>
            {/each}
        </div>
    </section>
</main>

<style>
    .gallery {
        padding: var(--space-6);
        display: flex;
        flex-direction: column;
        gap: var(--space-6);
        height: 100vh;
        overflow-y: auto;
        background: var(--bg-app);
        color: var(--text-main);
        font-family: var(--font-sans);
    }

    .gallery-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: var(--space-4);
    }

    section {
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
        padding: var(--space-4);
        background: var(--bg-panel);
        border: var(--border-width) solid var(--border);
        border-radius: var(--radius-lg);
    }

    .row {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: var(--space-2);
    }

    .scale {
        display: flex;
        flex-wrap: wrap;
        align-items: baseline;
        gap: var(--space-4);
    }

    .badge {
        padding: var(--space-0-5) var(--space-2);
        border: var(--border-width) solid transparent;
        border-radius: var(--radius-md);
        font-size: var(--font-sm);
        font-weight: var(--font-weight-bold);
    }

    .space {
        height: var(--space-4);
        background: var(--accent);
    }

    .radius {
        padding: var(--space-2) var(--space-4);
        background: var(--bg-panel-hover);
        font-size: var(--font-sm);
    }

    .swatches {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
        gap: var(--space-2);
    }

    .swatch {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        font-size: var(--font-sm);
    }

    .chip {
        width: 28px;
        height: 20px;
        border-radius: var(--radius-md);
        border: var(--border-width) solid var(--border);
    }

    code {
        font-family: var(--font-mono);
    }
</style>

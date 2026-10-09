<script lang="ts">
    import { onMount } from "svelte";
    import { Palette, FolderOpen, Download, RefreshCw, Trash2, Copy, TriangleAlert, Globe } from "lucide-svelte";
    import { confirm } from "@tauri-apps/plugin-dialog";
    import type { MappingState } from "../../features/mapping/mapping-state.svelte";
    import { deleteTheme, importTheme, openThemesFolder } from "../../infrastructure/tauri/commands";
    import { os } from "../../infrastructure/os";
    import {
        BUILTIN_THEMES,
        CUSTOM_PREFIX,
        SYSTEM_THEME,
        buildThemeTemplate,
        documentTokenReader,
        refreshUserThemes,
        themeStore,
    } from "../../theme";
    import Button from "../../components/ui/Button.svelte";
    import Select from "../../components/ui/Select.svelte";

    const { mapping }: { mapping: MappingState } = $props();

    const USER_THEMES_HEADER = "__my-themes";
    const THEMES_REPOSITORY = "https://github.com/NextTabletDriver/Themes";

    let status = $state<string | null>(null);

    const options = $derived([
        { value: SYSTEM_THEME, label: "System" },
        ...BUILTIN_THEMES.map((t) => ({ value: t.configValue, label: t.label })),
        ...(themeStore.userThemes.length > 0 ? [{ value: USER_THEMES_HEADER, label: "- My themes -", disabled: true }] : []),
        ...themeStore.userThemes.map((t) => ({
            value: `${CUSTOM_PREFIX}${t.id}`,
            label: t.definition?.metadata.name ?? t.fileName,
            disabled: t.definition === undefined,
        })),
    ]);

    const selectedUserTheme = $derived(
        themeStore.userThemes.find((t) => `${CUSTOM_PREFIX}${t.id}` === themeStore.selection),
    );
    const brokenThemes = $derived(themeStore.userThemes.filter((t) => t.definition === undefined));
    const themeNotices = $derived([
        ...(themeStore.resolved?.fallbackReason === undefined ? [] : [themeStore.resolved.fallbackReason]),
        ...themeStore.warnings,
    ]);

    onMount(() => {
        void refreshUserThemes();
    });

    function select(value: string): void {
        if (mapping.config === null || value === USER_THEMES_HEADER) return;
        mapping.config.theme = value;
        mapping.markDirty();
    }

    async function handleImport(): Promise<void> {
        try {
            const imported = await importTheme();
            if (imported === null) return;
            await refreshUserThemes();
            const entry = themeStore.userThemes.find((t) => t.id === imported.id);
            if (entry?.definition === undefined) {
                status = `"${imported.file_name}" was imported but cannot be used: see the list below.`;
                return;
            }
            select(`${CUSTOM_PREFIX}${imported.id}`);
            status = `Imported "${entry.definition.metadata.name}".`;
        } catch (e) {
            status = `Import failed: ${String(e)}`;
        }
    }

    async function handleDelete(): Promise<void> {
        const entry = selectedUserTheme;
        if (entry === undefined) return;
        const name = entry.definition?.metadata.name ?? entry.fileName;
        if (!(await confirm(`Delete the theme "${name}"? The file will be removed from the themes folder.`))) return;
        try {
            await deleteTheme(entry.id);
            select("Dark");
            await refreshUserThemes();
            status = `Deleted "${name}".`;
        } catch (e) {
            status = `Delete failed: ${String(e)}`;
        }
    }

    async function handleCopyTemplate(): Promise<void> {
        const base = themeStore.resolved?.render.scheme ?? "dark";
        const template = buildThemeTemplate(documentTokenReader(document.documentElement), base);
        try {
            await navigator.clipboard.writeText(template);
            status = "Template copied: paste it into a new .json file in the themes folder.";
        } catch {
            status = "Could not access the clipboard.";
        }
    }

    async function handleReload(): Promise<void> {
        await refreshUserThemes();
        status = "Themes reloaded.";
    }
</script>

<div class="section">
    <div class="section-header">
        <Palette size={20} />
        <h2>Themes</h2>
    </div>

    {#if mapping.config}
    <div class="flex-column settings-group">
        <div class="input-group">
            <label for="current_theme">Current Theme</label>
            <div class="theme-select-wrapper">
                <Select onchange={select} {options} value={themeStore.selection} />
            </div>
        </div>

        <div class="theme-actions">
            <div class="action-buttons">
                <Button onclick={() => { void handleImport(); }}><Download size={16} /> Import theme...</Button>
                <Button onclick={() => { void handleReload(); }}><RefreshCw size={16} /> Reload</Button>
                <Button onclick={() => { void openThemesFolder(); }}><FolderOpen size={16} /> Open Themes Folder</Button>
                <Button onclick={() => { void handleCopyTemplate(); }}><Copy size={16} /> Copy as template</Button>
                {#if selectedUserTheme !== undefined}
                    <Button onclick={() => { void handleDelete(); }} variant="danger-ghost"><Trash2 size={16} /> Delete</Button>
                {/if}
                <Button onclick={() => { void os.openUrl(THEMES_REPOSITORY); }}><Globe size={16} /> Browse Online Themes</Button>
            </div>
            {#if status !== null}
                <p class="status" role="status">{status}</p>
            {/if}
        </div>

        {#if themeNotices.length > 0}
            <ul class="notices" aria-label="Theme warnings">
                {#each themeNotices as notice (notice)}
                    <li><TriangleAlert size={14} /> {notice}</li>
                {/each}
            </ul>
        {/if}

        {#if brokenThemes.length > 0}
            <div class="broken">
                <h3>Themes that could not be loaded</h3>
                <ul>
                    {#each brokenThemes as theme (theme.id)}
                        <li>
                            <strong>{theme.fileName}</strong>
                            <ul>
                                {#each theme.errors as issue (`${issue.path}:${issue.message}`)}
                                    <li><code>{issue.path === "" ? "file" : issue.path}</code> {issue.message}</li>
                                {/each}
                            </ul>
                        </li>
                    {/each}
                </ul>
            </div>
        {/if}
    </div>
    {/if}
</div>

<style>
    .section-header {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        margin-bottom: var(--space-4);
        color: var(--text-active);
    }

    .section-header h2 {
        font-size: var(--font-xl);
        font-weight: var(--font-weight-bold);
        margin: 0;
    }

    .settings-group {
        padding-left: var(--space-7);
        gap: var(--space-4);
    }

    .flex-column {
        display: flex;
        flex-direction: column;
    }

    .input-group {
        display: flex;
        align-items: center;
        gap: var(--space-4);
    }

    .input-group label {
        font-size: var(--font-md);
        color: var(--text-active);
        font-weight: var(--font-weight-bold);
        min-width: 120px;
    }

    .theme-select-wrapper {
        width: 200px;
    }

    .action-buttons {
        display: flex;
        flex-wrap: wrap;
        gap: var(--space-2);
    }

    .status {
        margin-top: var(--space-2);
        font-size: var(--font-base);
        color: var(--text-muted);
    }

    .notices {
        list-style: none;
        display: flex;
        flex-direction: column;
        gap: var(--space-1);
        font-size: var(--font-base);
        color: var(--warning);
    }

    .notices li {
        display: flex;
        align-items: center;
        gap: var(--space-2);
    }

    .broken {
        border: var(--border-width) solid var(--error-border);
        background: var(--error-bg);
        border-radius: var(--radius-md);
        padding: var(--space-3) var(--space-4);
        font-size: var(--font-base);
    }

    .broken h3 {
        font-size: var(--font-md);
        font-weight: var(--font-weight-bold);
        color: var(--error);
        margin-bottom: var(--space-2);
    }

    .broken ul {
        list-style: none;
        display: flex;
        flex-direction: column;
        gap: var(--space-1);
    }

    .broken ul ul {
        padding-left: var(--space-4);
        color: var(--text-muted);
    }

    code {
        font-family: var(--font-mono);
        color: var(--text-main);
    }
</style>

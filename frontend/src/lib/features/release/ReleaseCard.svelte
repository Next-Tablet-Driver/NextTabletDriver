<script lang="ts">
    import { ArrowUpRight } from "lucide-svelte";
    import { open as openUrl } from "@tauri-apps/plugin-shell";

  interface Release {
        tag_name: string;
        body: string | null;
        published_at: string | null;
    }

    interface Props {
        release: Release;
        isLatest?: boolean;
    }

    const { release, isLatest = false }: Props = $props();

    function formatDate(dateStr: string | null): string {
        if (dateStr === null || dateStr === "") return "";
        return new Date(dateStr).toLocaleDateString("en-US", {
            month: "long",
            day: "numeric",
            year: "numeric"
        });
    }

    interface ParsedBody {
        items: string[];
        contributors: string[];
        fullChangelog: string | null;
    }

    function parseBody(body: string | null): ParsedBody {
        if (body === null || body === "") return { items: [], contributors: [], fullChangelog: null };
        
        const lines = body.split('\n');
        const items: string[] = [];
        let fullChangelog: string | null = null;
        const users: string[] = [];

        const userRegex = /@([a-zA-Z0-9-]+)/g;

        const processLine = (text: string): void => {
            text = text.replace(/ in https:\/\/github\.com\/\S+/g, "");
            items.push(text);
            let match;
            while ((match = userRegex.exec(text)) !== null) {
                const user = match[1];
                if (!users.includes(user)) {
                    users.push(user);
                }
            }
        };

        for (let line of lines) {
            line = line.trim();
            if (line === "") continue;
            
            if (line.includes("**Full Changelog**:")) {
                fullChangelog = line.replace("**Full Changelog**:", "").trim();
                continue;
            }

            if (line.startsWith('#')) continue;

            if (line.startsWith('*') || line.startsWith('-')) {
                processLine(line.substring(1).trim());
            } else if (line.length > 5) {
                processLine(line);
            }
        }

        return {
            items,
            contributors: users,
            fullChangelog
        };
    }

    interface ParsedPart {
        type: 'text' | 'url' | 'user';
        content: string;
        display?: string;
    }

    function processUrlMatch(url: string): ParsedPart {
        let display = url;
        const prMatch = /\/pull\/(\d+)/.exec(url);
        if (prMatch?.[1] !== undefined) display = `#${prMatch[1]}`;
        const compareMatch = /\/compare\/(.*)/.exec(url);
        if (compareMatch?.[1] !== undefined) display = compareMatch[1];
        return { type: 'url', content: url, display };
    }

    function parseTextWithLinks(text: string): ParsedPart[] {
        const regex = /(https?:\/\/[^\s]+)|@([a-zA-Z0-9-]+)/g;
        const parts: ParsedPart[] = [];
        let lastIndex = 0;
        let match;

        while ((match = regex.exec(text)) !== null) {
            if (match.index > lastIndex) {
                parts.push({ type: 'text', content: text.substring(lastIndex, match.index) });
            }
            if ((match[1] as string | undefined) !== undefined) {
                parts.push(processUrlMatch(match[1]));
            } else if ((match[2] as string | undefined) !== undefined) {
                parts.push({ type: 'user', content: match[2] });
            }
            lastIndex = regex.lastIndex;
        }
        if (lastIndex < text.length) {
            parts.push({ type: 'text', content: text.substring(lastIndex) });
        }
        return parts;
    }

    const parsed = $derived(parseBody(release.body));
</script>

<div class="release-card" class:is-latest={isLatest}>
    <div class="card-header">
        <div class="title-group">
            <h2>{release.tag_name}</h2>
            {#if isLatest}
                <span class="badge latest">Latest</span>
            {/if}
        </div>
        <span class="date">{formatDate(release.published_at)}</span>
    </div>

    {#if parsed.items.length > 0}
        <div class="changelog">
            {#each parsed.items as item, itemIndex (itemIndex)}
                <div class="changelog-item">
                    <span class="bullet"></span>
                    <span class="item-text">
                        {#each parseTextWithLinks(item) as part, index (index)}
                            {#if part.type === 'url'}
                                <a class="link-url" href={part.content} onclick={(e) => { e.preventDefault(); void openUrl(part.content); }}>
                                    {part.display}
                                </a>
                            {:else if part.type === 'user'}
                                <a class="user-mention" href={`https://github.com/${part.content}`} onclick={(e) => { e.preventDefault(); void openUrl(`https://github.com/${part.content}`); }}>@{part.content}</a>
                            {:else}
                                {part.content}
                            {/if}
                        {/each}
                    </span>
                </div>
            {/each}
        </div>
    {/if}

    <div class="card-footer">
        {#if parsed.contributors.length > 0}
            <div class="contributors">
                {#each parsed.contributors.slice(0, 8) as user (user)}
                    <a aria-label="{user}" href={`https://github.com/${user}`} onclick={(e) => { e.preventDefault(); void openUrl(`https://github.com/${user}`); }}>
                        <img class="avatar" alt="{user}" src="https://github.com/{user}.png?size=40" title="@{user}"/>
                    </a>
                {/each}
                {#if parsed.contributors.length > 8}
                    <div class="avatar-more">+{parsed.contributors.length - 8}</div>
                {/if}
            </div>
        {:else}
            <div></div>
        {/if}
        
        <div class="actions">
            {#if parsed.fullChangelog !== null}
                {@const url = parsed.fullChangelog}
                <a class="btn-link" href={url} onclick={(e) => { e.preventDefault(); void openUrl(url); }}>
                    <!-- <GitCommit size={14} /> -->
                    <span>View Commits</span>
                </a>
            {/if}
            <a class="btn-link" href={`https://github.com/Next-Tablet-Driver/NextTabletDriver/releases/tag/${release.tag_name}`} onclick={(e) => { e.preventDefault(); void openUrl(`https://github.com/Next-Tablet-Driver/NextTabletDriver/releases/tag/${release.tag_name}`); }}>
                <!-- <Github size={14} /> -->
                <span>GitHub Release</span>
                <ArrowUpRight size={14} />
            </a>
        </div>
    </div>
</div>

<style>
    .release-card {
        background-color: var(--bg-panel);
        border: var(--border-width) solid var(--border);
        border-radius: var(--radius-md, var(--radius-xl));
        padding: var(--space-6);
        display: flex;
        flex-direction: column;
        gap: var(--space-5);
        transition: border-color var(--transition-moderate), box-shadow var(--transition-moderate);
    }

    .release-card:hover {
        border-color: var(--border-hover, var(--overlay-15));
    }

    .card-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
    }

    .title-group {
        display: flex;
        align-items: center;
        gap: var(--space-3);
    }

    .card-header h2 {
        margin: 0;
        font-size: var(--font-3xl);
        font-weight: var(--font-weight-heavy);
        color: var(--text-main);
        letter-spacing: -0.5px;
    }

    .badge {
        font-size: var(--font-sm);
        font-weight: var(--font-weight-bold);
        text-transform: uppercase;
        padding: var(--space-0-5) var(--space-2);
        background-color: var(--border);
        color: var(--text-muted);
    }

    .date {
        font-size: var(--font-md);
        color: var(--text-muted);
        font-weight: var(--font-weight-medium);
    }

    .changelog {
        display: flex;
        flex-direction: column;
        gap: var(--space-3);
    }

    .changelog-item {
        display: flex;
        align-items: flex-start;
        gap: var(--space-3);
        font-size: var(--font-lg);
        line-height: 1.6;
        color: var(--text-muted);
    }

    .bullet {
        margin-top: var(--space-2);
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background-color: var(--text-muted);
        opacity: 0.5;
        flex-shrink: 0;
    }

    .item-text {
        color: var(--text-muted);
    }

    .link-url {
        color: var(--accent);
        text-decoration: none;
        background: color-mix(in srgb, var(--accent) 8%, transparent);
        border: var(--border-width) solid color-mix(in srgb, var(--accent) 20%, transparent);
        padding: var(--space-0-5) var(--space-1-5);
        border-radius: var(--radius-md);
        font-family: var(--font-mono, monospace);
        font-size: var(--font-sm);
        font-weight: var(--font-weight-bold);
        margin: 0 var(--space-1);
        transition: all var(--transition-moderate) ease;
    }

    .link-url:hover {
        background: color-mix(in srgb, var(--accent) 15%, transparent);
        border-color: color-mix(in srgb, var(--accent) 40%, transparent);
        transform: translateY(-1px);
    }

    .user-mention {
        color: var(--text-main);
        font-weight: var(--font-weight-bold);
        text-decoration: none;
        transition: color var(--transition-moderate);
    }

    .user-mention:hover {
        color: var(--accent);
    }

    .card-footer {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-top: var(--space-2);
        padding-top: var(--space-5);
        border-top: var(--border-width) solid var(--border);
    }

    .contributors {
        display: flex;
        align-items: center;
    }

    .avatar {
        width: 28px;
        height: 28px;
        border-radius: 50%;
        border: 2px solid var(--bg-panel);
        margin-left: calc(var(--space-2) * -1);
        cursor: pointer;
        transition: transform var(--transition-moderate), z-index 0s;
        position: relative;
    }

    .avatar:first-child {
        margin-left: 0;
    }

    .avatar:hover {
        transform: translateY(-2px) scale(1.1);
        z-index: 10;
    }

    .avatar-more {
        width: 28px;
        height: 28px;
        border-radius: 50%;
        border: 2px solid var(--bg-panel);
        background-color: var(--border);
        margin-left: calc(var(--space-2) * -1);
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: var(--font-sm);
        font-weight: var(--font-weight-bold);
        color: var(--text-main);
        z-index: 1;
    }

    .actions {
        display: flex;
        gap: var(--space-3);
    }

    .btn-link {
        display: flex;
        align-items: center;
        gap: var(--space-1-5);
        padding: var(--space-1-5) var(--space-3);
        border-radius: var(--radius-md-plus);
        background-color: var(--bg-app);
        border: var(--border-width) solid var(--border);
        color: var(--text-main);
        font-size: var(--font-md);
        font-weight: var(--font-weight-medium);
        text-decoration: none;
        transition: all var(--transition-moderate);
    }

    .btn-link:hover {
        background-color: var(--border);
        border-color: var(--overlay-20);
    }
</style>

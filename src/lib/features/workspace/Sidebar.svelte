<script lang="ts">
    import type { Folder } from '$lib/api'
    import Button from '$lib/design/Button.svelte'
    import Icon from '$lib/design/Icon.svelte'
    import { t } from '$lib/i18n'
    import { folderIcon, folderLabel, groupFolders } from './workspace'

    type Props = {
        // The folders of the account that is open.
        folders: Folder[]
        selectedFolderId: number | null
        onselect: (folder: Folder) => void
        oncompose: () => void
    }

    let { folders, selectedFolderId, onselect, oncompose }: Props = $props()

    let groups = $derived(groupFolders(folders))

    // Read out with the name, because the rail hides the text.
    const describe = (folder: Folder): string => {
        if (folder.unread === 0) {
            return folderLabel(folder)
        }

        return t('sidebar.folderUnread', { name: folderLabel(folder), count: String(folder.unread) })
    }
</script>

{#snippet row(folder: Folder, indent: number)}
    <li>
        <button
            class="folder"
            class:selected={folder.id === selectedFolderId}
            class:unread={folder.unread > 0}
            class:inbox={folder.role === 'Inbox'}
            type="button"
            title={folderLabel(folder)}
            aria-label={describe(folder)}
            aria-current={folder.id === selectedFolderId}
            style:--indent={indent}
            onclick={() => onselect(folder)}
        >
            <Icon name={folderIcon(folder)} />
            <span class="name">{folderLabel(folder)}</span>
            {#if folder.unread > 0}
                <span class="count">{folder.unread}</span>
            {/if}
        </button>
    </li>
{/snippet}

<aside class="sidebar">
    <div class="compose">
        <Button variant="accent" size="large" icon="compose" onclick={oncompose}>{t('toolbar.compose')}</Button>
    </div>

    <nav aria-label={t('sidebar.folders')}>
        <ul>
            {#each groups.main as folder (folder.id)}
                {@render row(folder, 0)}
            {/each}
        </ul>

        {#if groups.custom.length > 0}
            <h2><span>{t('sidebar.folders')}</span></h2>
            <ul>
                {#each groups.custom as entry (entry.folder.id)}
                    {@render row(entry.folder, entry.indent)}
                {/each}
            </ul>
        {/if}
    </nav>
</aside>

<style>
    .sidebar {
        container: sidebar / inline-size;
        display: flex;
        flex-direction: column;
        min-height: 0;
        color: var(--glass-text);
    }

    .compose {
        display: grid;
        padding: var(--space-2);
    }

    nav {
        flex: 1;
        min-height: 0;
        overflow-y: auto;
        padding: 0 var(--space-2) var(--space-2);
    }

    ul {
        display: grid;
        gap: 1px;
        margin: 0;
        padding: 0;
        list-style: none;
    }

    /* The second group is announced by a ruled caption, like a group box. */
    h2 {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        margin: var(--space-3) var(--space-1) var(--space-1);
        font-size: var(--font-size-small);
        font-weight: 400;
        opacity: 0.75;
        text-shadow: var(--glass-text-shadow);
    }

    h2::after {
        content: '';
        flex: 1;
        border-top: 1px solid currentColor;
        opacity: 0.4;
    }

    .folder {
        position: relative;
        display: flex;
        align-items: center;
        gap: var(--space-2);
        width: 100%;
        min-height: 30px;
        padding: var(--space-1) var(--space-2);
        padding-left: calc(var(--space-2) + var(--indent) * var(--space-4));
        border: 1px solid transparent;
        border-radius: var(--radius-control);
        background: none;
        color: inherit;
        text-align: left;
        cursor: default;
    }

    .folder:hover {
        border-color: var(--row-hover-border);
        background: var(--row-hover-face);
        color: var(--text);
    }

    .folder.selected {
        border-color: var(--row-selected-border);
        background: var(--row-selected-face);
        color: var(--text);
    }

    .folder:focus-visible {
        outline: none;
        box-shadow: var(--focus-glow);
    }

    .name {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .unread .name {
        font-weight: 600;
    }

    .count {
        flex: none;
        min-width: 22px;
        padding: 0 var(--space-1);
        border: 1px solid transparent;
        border-radius: var(--radius-panel);
        background: var(--badge-quiet-face);
        font-size: var(--font-size-small);
        font-weight: 600;
        text-align: center;
    }

    /* New mail arrives in the inbox: its count is the one that stands out. */
    .inbox .count {
        border-color: var(--badge-border);
        background: var(--badge-face);
        color: var(--badge-text);
    }

    /* The rail: too narrow for names, so icons only. */
    @container sidebar (max-width: 100px) {
        .compose {
            padding: var(--space-2) var(--space-1);
        }

        .compose :global(.label),
        .name,
        h2 span {
            display: none;
        }

        nav {
            padding: 0 var(--space-1) var(--space-2);
        }

        h2 {
            margin: var(--space-2) var(--space-1);
        }

        .folder {
            justify-content: center;
            padding: var(--space-1) 0;
        }

        .count {
            display: none;
        }

        .unread::after {
            content: '';
            position: absolute;
            top: var(--space-1);
            right: var(--space-1);
            width: 8px;
            height: 8px;
            border: 1px solid var(--badge-border);
            border-radius: 50%;
            background: var(--badge-face);
        }
    }
</style>

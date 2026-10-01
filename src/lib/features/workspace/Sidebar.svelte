<script lang="ts">
    import type { Account, Folder } from '$lib/api'
    import Button from '$lib/design/Button.svelte'
    import { t } from '$lib/i18n'
    import { folderIndent, folderLabel } from './workspace'

    type Props = {
        accounts: Account[]
        folders: Record<number, Folder[]>
        selectedFolderId: number | null
        onselect: (folder: Folder) => void
        onadd: () => void
        onremove: (account: Account) => void
    }

    let { accounts, folders, selectedFolderId, onselect, onadd, onremove }: Props = $props()

    // The account whose removal is waiting for confirmation.
    let removing = $state<number | null>(null)

    const confirmRemove = (account: Account) => {
        removing = null
        onremove(account)
    }
</script>

<nav class="sidebar" aria-label={t('sidebar.folders')}>
    <div class="accounts">
        {#each accounts as account (account.id)}
            <section>
                <header>
                    <span class="email" title={account.settings.email}>{account.settings.email}</span>
                    <button
                        class="remove"
                        type="button"
                        title={t('sidebar.removeAccount')}
                        aria-label={`${t('sidebar.removeAccount')}: ${account.settings.email}`}
                        onclick={() => removing = account.id}
                    >×</button>
                </header>

                {#if removing === account.id}
                    <div class="confirm" role="alertdialog" aria-label={t('sidebar.removeAccount')}>
                        <p>{t('sidebar.removeConfirm', { email: account.settings.email })}</p>
                        <div class="choices">
                            <Button onclick={() => confirmRemove(account)}>{t('sidebar.remove')}</Button>
                            <Button onclick={() => removing = null}>{t('sidebar.keep')}</Button>
                        </div>
                    </div>
                {/if}

                <ul>
                    {#each folders[account.id] ?? [] as folder (folder.id)}
                        {#if folder.selectable}
                            <li>
                                <button
                                    class="folder"
                                    class:selected={folder.id === selectedFolderId}
                                    type="button"
                                    aria-current={folder.id === selectedFolderId}
                                    style:padding-left={`calc(var(--space-3) + ${folderIndent(folder)} * var(--space-4))`}
                                    onclick={() => onselect(folder)}
                                >
                                    <span class="name" class:unread={folder.unread > 0}>{folderLabel(folder)}</span>
                                    {#if folder.unread > 0}
                                        <span class="count">{folder.unread}</span>
                                    {/if}
                                </button>
                            </li>
                        {/if}
                    {/each}
                </ul>
            </section>
        {/each}
    </div>

    <footer>
        <Button onclick={onadd}>{t('sidebar.addAccount')}</Button>
    </footer>
</nav>

<style>
    .sidebar {
        display: flex;
        flex-direction: column;
        min-height: 0;
        color: var(--glass-text);
    }

    .accounts {
        flex: 1;
        min-height: 0;
        overflow-y: auto;
        padding: var(--space-2);
    }

    section + section {
        margin-top: var(--space-4);
    }

    header {
        display: flex;
        align-items: center;
        gap: var(--space-1);
        padding: var(--space-1) var(--space-2);
        font-weight: 600;
        text-shadow: var(--glass-text-shadow);
    }

    .email {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .remove {
        width: 20px;
        height: 20px;
        padding: 0;
        border: 1px solid transparent;
        border-radius: var(--radius-control);
        background: none;
        color: inherit;
        line-height: 1;
        opacity: 0.6;
    }

    .remove:hover,
    .remove:focus-visible {
        border-color: var(--control-border);
        background: var(--control-face);
        color: var(--control-text);
        opacity: 1;
        outline: none;
    }

    .confirm {
        margin: var(--space-1) 0 var(--space-2);
        padding: var(--space-2);
        border: 1px solid var(--warning-border);
        border-radius: var(--radius-control);
        background: var(--warning-background);
        color: var(--text);
    }

    .choices {
        display: flex;
        gap: var(--space-2);
        margin-top: var(--space-2);
    }

    ul {
        margin: 0;
        padding: 0;
        list-style: none;
    }

    .folder {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        width: 100%;
        padding: var(--space-1) var(--space-2);
        border: 1px solid transparent;
        border-radius: var(--radius-control);
        background: none;
        color: inherit;
        text-align: left;
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

    .name.unread {
        font-weight: 600;
    }

    .count {
        font-size: var(--font-size-small);
        font-weight: 600;
        color: var(--unread-mark);
    }

    footer {
        padding: var(--space-2);
    }
</style>

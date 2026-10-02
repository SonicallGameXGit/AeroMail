<script lang="ts">
    import type { Account, Folder } from '$lib/api'
    import Avatar from '$lib/design/Avatar.svelte'
    import Icon from '$lib/design/Icon.svelte'
    import Menu from '$lib/design/Menu.svelte'
    import MenuItem from '$lib/design/MenuItem.svelte'
    import { t } from '$lib/i18n'
    import { accountLabel, inboxUnread } from './workspace'

    type Props = {
        accounts: Account[]
        folders: Record<number, Folder[]>
        // The account whose mail is open.
        current: Account
        onselect: (account: Account) => void
        onadd: () => void
        // Removing an account and everything else about the app is in the settings.
        onsettings: () => void
    }

    let { accounts, folders, current, onselect, onadd, onsettings }: Props = $props()
</script>

<Menu
    label={t('account.current', { email: current.settings.email })}
    variant="normal"
    align="end"
>
    {#snippet button()}
        <span class="chip">
            <Avatar name={accountLabel(current)} seed={current.settings.email} size="small" />
            <span class="chip-name">{accountLabel(current)}</span>
            <Icon name="chevron" size={14} />
        </span>
    {/snippet}

    {#snippet children(close)}
        {#each accounts as account (account.id)}
            {@const unread = inboxUnread(folders[account.id] ?? [])}
            <MenuItem
                current={account.id === current.id}
                onclick={() => {
                    close()
                    onselect(account)
                }}
            >
                <span class="entry">
                    <Avatar name={accountLabel(account)} seed={account.settings.email} />
                    <span class="who">
                        <span class="name">{accountLabel(account)}</span>
                        {#if accountLabel(account) !== account.settings.email}
                            <span class="email">{account.settings.email}</span>
                        {/if}
                    </span>
                    {#if unread > 0}
                        <span class="count">{unread}</span>
                    {/if}
                </span>
            </MenuItem>
        {/each}

        <hr />

        <MenuItem
            icon="plus"
            onclick={() => {
                close()
                onadd()
            }}
        >{t('account.add')}</MenuItem>

        <hr />

        <MenuItem
            icon="settings"
            onclick={() => {
                close()
                onsettings()
            }}
        >{t('settings.title')}</MenuItem>
    {/snippet}
</Menu>

<style>
    .chip {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        min-width: 0;
    }

    .chip-name {
        max-width: 260px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .entry {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        min-width: 0;
    }

    .who {
        flex: 1;
        display: grid;
        min-width: 0;
    }

    .name,
    .email {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .name {
        font-weight: 600;
    }

    .email {
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    .count {
        flex: none;
        min-width: 22px;
        padding: 0 var(--space-1);
        border: 1px solid var(--badge-border);
        border-radius: var(--radius-panel);
        background: var(--badge-face);
        color: var(--badge-text);
        font-size: var(--font-size-small);
        font-weight: 600;
        text-align: center;
    }

    hr {
        width: 100%;
        margin: var(--space-1) 0;
        border: 0;
        border-top: 1px solid var(--row-separator);
    }
</style>

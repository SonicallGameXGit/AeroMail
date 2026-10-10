<script lang="ts" module>
    export type Category = 'accounts' | 'appearance' | 'privacy' | 'notifications'

    // The category opened last stays open the next time, while the app runs.
    let lastCategory: Category = 'accounts'
</script>

<script lang="ts">
    import type { Account, AppSettings } from '$lib/api'
    import Avatar from '$lib/design/Avatar.svelte'
    import Button from '$lib/design/Button.svelte'
    import CheckField from '$lib/design/CheckField.svelte'
    import Icon from '$lib/design/Icon.svelte'
    import type { IconName } from '$lib/design/icons'
    import NotificationOptions from '$lib/features/notifications/NotificationOptions.svelte'
    import { accountLabel } from '$lib/features/workspace/workspace'
    import ThemePicker from './ThemePicker.svelte'
    import { t } from '$lib/i18n'

    type Props = {
        accounts: Account[]
        settings: AppSettings
        // Every change is applied at once; there is nothing to save.
        onchange: (settings: AppSettings) => void
        onadd: () => void
        onremove: (account: Account) => void
        onclose: () => void
        onresetnotifications: () => void
    }

    let { accounts, settings, onchange, onadd, onremove, onclose, onresetnotifications }: Props = $props()

    const categories: { id: Category, icon: IconName }[] = [
        { id: 'accounts', icon: 'account' },
        { id: 'appearance', icon: 'appearance' },
        { id: 'privacy', icon: 'privacy' },
        { id: 'notifications', icon: 'bell' },
    ]

    let category = $state<Category>(lastCategory)
    const id = $props.id()
    const tabs: Record<string, HTMLButtonElement> = {}

    const open = (next: Category) => {
        category = next
        lastCategory = next
    }

    // Up and down walk the list, Home and End jump to its ends, as in any
    // list of tabs; the category opens as soon as it is reached.
    const walk = (event: KeyboardEvent, index: number) => {
        const last = categories.length - 1
        const moves: Record<string, number> = {
            ArrowDown: index === last ? 0 : index + 1,
            ArrowUp: index === 0 ? last : index - 1,
            Home: 0,
            End: last,
        }
        const target = categories[moves[event.key] ?? -1]
        if (target === undefined) {
            return
        }

        event.preventDefault()
        open(target.id)
        tabs[target.id]?.focus()
    }
</script>

<div class="settings">
    <div class="tabs" role="tablist" aria-orientation="vertical" aria-label={t('settings.categories')}>
        {#each categories as entry, index (entry.id)}
            <button
                bind:this={tabs[entry.id]}
                class="tab"
                class:selected={entry.id === category}
                type="button"
                role="tab"
                id={`${id}-tab-${entry.id}`}
                aria-selected={entry.id === category}
                aria-controls={`${id}-panel`}
                tabindex={entry.id === category ? 0 : -1}
                onclick={() => open(entry.id)}
                onkeydown={(event) => walk(event, index)}
            >
                <Icon name={entry.icon} size={20} />
                <span>{t(`settings.${entry.id}`)}</span>
            </button>
        {/each}
    </div>

    <!-- Focusable, so a long category can be scrolled from the keyboard. -->
    <div class="panel" id={`${id}-panel`} role="tabpanel" tabindex="0" aria-labelledby={`${id}-tab-${category}`}>
        <h2>{t(`settings.${category}`)}</h2>

        {#if category === 'accounts'}
            <ul>
                {#each accounts as account (account.id)}
                    <li>
                        <Avatar name={accountLabel(account)} seed={account.settings.email} />
                        <span class="who">
                            <span class="name">{accountLabel(account)}</span>
                            {#if accountLabel(account) !== account.settings.email}
                                <span class="email">{account.settings.email}</span>
                            {/if}
                        </span>
                        <Button
                            tone="danger"
                            icon="trash"
                            label={`${t('account.remove')}: ${account.settings.email}`}
                            onclick={() => onremove(account)}
                        >{t('account.removeYes')}</Button>
                    </li>
                {/each}
            </ul>
            <div class="row">
                <Button icon="plus" onclick={onadd}>{t('account.add')}</Button>
            </div>
            <CheckField
                label={t('settings.switchToNewAccount')}
                hint={t('settings.switchToNewAccountHint')}
                checked={settings.switchToNewAccount}
                onchange={(switchToNewAccount) => onchange({ ...settings, switchToNewAccount })}
            />
        {:else if category === 'appearance'}
            <ThemePicker theme={settings.theme} onchange={(theme) => onchange({ ...settings, theme })} />
        {:else if category === 'privacy'}
            <CheckField
                label={t('settings.pictures')}
                hint={t('settings.picturesHint')}
                checked={settings.senderPictures}
                onchange={(senderPictures) => onchange({ ...settings, senderPictures })}
            />
            <CheckField
                label={t('settings.remoteImages')}
                hint={t('settings.remoteImagesHint')}
                checked={settings.remoteImages}
                onchange={(remoteImages) => onchange({ ...settings, remoteImages })}
            />
        {:else}
            <NotificationOptions
                settings={settings.notifications}
                onchange={(notifications) => onchange({ ...settings, notifications })}
                onreset={onresetnotifications}
            />
        {/if}
    </div>

    <div class="footer">
        <Button variant="accent" onclick={onclose}>{t('settings.done')}</Button>
    </div>
</div>

<style>
    /* The window keeps one size whatever category is open, so switching
       between them does not make it jump; a long category scrolls. */
    .settings {
        display: grid;
        grid-template-columns: 180px minmax(0, 1fr);
        grid-template-rows: minmax(0, 1fr) auto;
        gap: var(--space-3) var(--space-5);
        height: min(560px, 72vh);
        min-height: 0;
    }

    .tabs {
        display: grid;
        align-content: start;
        gap: 1px;
        padding: var(--space-1);
        border: 1px solid var(--surface-border);
        border-radius: var(--radius-control);
        background: var(--surface-sunken);
    }

    .tab {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        width: 100%;
        min-height: 34px;
        padding: var(--space-1) var(--space-2);
        border: 1px solid transparent;
        border-radius: var(--radius-control);
        background: none;
        color: var(--text);
        text-align: left;
        cursor: default;
    }

    .tab:hover {
        border-color: var(--row-hover-border);
        background: var(--row-hover-face);
    }

    .tab.selected {
        border-color: var(--row-selected-border);
        background: var(--row-selected-face);
        font-weight: 600;
    }

    .tab:focus-visible,
    .panel:focus-visible {
        outline: none;
        box-shadow: var(--focus-glow);
    }

    .panel {
        display: grid;
        align-content: start;
        gap: var(--space-3);
        min-height: 0;
        overflow-y: auto;
        /* Room for focus glows at the edges of the scrolled area. */
        padding: 0 var(--space-2) var(--space-2) 0;
    }

    h2 {
        padding-bottom: var(--space-1);
        border-bottom: 1px solid var(--row-separator);
        font-size: var(--font-size-title);
        font-weight: 400;
        color: var(--text-link);
    }

    ul {
        display: grid;
        gap: var(--space-2);
        margin: 0;
        padding: 0;
        list-style: none;
    }

    li {
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

    .row {
        display: flex;
    }

    .footer {
        grid-column: 1 / -1;
        display: flex;
        justify-content: flex-end;
        padding-top: var(--space-3);
        border-top: 1px solid var(--row-separator);
    }
</style>

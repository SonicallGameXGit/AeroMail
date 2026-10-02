<script lang="ts">
    import type { Account, AppSettings } from '$lib/api'
    import Avatar from '$lib/design/Avatar.svelte'
    import Button from '$lib/design/Button.svelte'
    import CheckField from '$lib/design/CheckField.svelte'
    import { accountLabel } from '$lib/features/workspace/workspace'
    import { t } from '$lib/i18n'

    type Props = {
        accounts: Account[]
        settings: AppSettings
        // Every change is applied at once; there is nothing to save.
        onchange: (settings: AppSettings) => void
        onadd: () => void
        onremove: (account: Account) => void
        onclose: () => void
    }

    let { accounts, settings, onchange, onadd, onremove, onclose }: Props = $props()
</script>

<div class="settings">
    <section aria-labelledby="settings-accounts">
        <h2 id="settings-accounts">{t('settings.accounts')}</h2>
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
    </section>

    <section aria-labelledby="settings-privacy">
        <h2 id="settings-privacy">{t('settings.privacy')}</h2>
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
    </section>

    <div class="row end">
        <Button variant="accent" onclick={onclose}>{t('settings.done')}</Button>
    </div>
</div>

<style>
    .settings,
    section {
        display: grid;
        gap: var(--space-3);
    }

    .settings {
        gap: var(--space-5);
    }

    h2 {
        padding-bottom: var(--space-1);
        border-bottom: 1px solid var(--row-separator);
        font-size: var(--font-size-body);
        font-weight: 600;
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

    .end {
        justify-content: flex-end;
    }
</style>

<script lang="ts">
    import type { Account, MailError } from '$lib/api'
    import Button from '$lib/design/Button.svelte'
    import Notice from '$lib/design/Notice.svelte'
    import SelectField from '$lib/design/SelectField.svelte'
    import TextField from '$lib/design/TextField.svelte'
    import { t } from '$lib/i18n'
    import { errorMessage, type ComposeForm } from './workspace'

    type Props = {
        accounts: Account[]
        form: ComposeForm
        sending: boolean
        error: MailError | null
        onchange: (form: ComposeForm) => void
        onsend: () => void
        ondiscard: () => void
    }

    let { accounts, form, sending, error, onchange, onsend, ondiscard }: Props = $props()

    // Cc and Bcc stay out of the way until asked for, or until they hold something.
    let copiesOpen = $state(false)
    let showCopies = $derived(copiesOpen || form.cc !== '' || form.bcc !== '')

    const accountOptions = $derived(accounts.map((account) => ({
        value: String(account.id),
        label: account.settings.email,
    })))

    const submit = (event: SubmitEvent) => {
        event.preventDefault()
        onsend()
    }

    const openCopies = () => {
        copiesOpen = true
    }
</script>

<form class="composer" novalidate onsubmit={submit}>
    {#if accounts.length > 1}
        <SelectField
            label={t('compose.from')}
            value={String(form.accountId)}
            options={accountOptions}
            onchange={(id) => onchange({ ...form, accountId: Number(id) })}
        />
    {/if}

    <div class="recipients">
        <TextField
            label={t('compose.to')}
            placeholder={t('compose.recipientsHint')}
            value={form.to}
            autofocus={form.to === ''}
            oninput={(to) => onchange({ ...form, to })}
        />
        {#if !showCopies}
            <Button variant="link" onclick={openCopies}>{t('compose.showCopies')}</Button>
        {/if}
    </div>

    {#if showCopies}
        <TextField label={t('compose.cc')} value={form.cc} oninput={(cc) => onchange({ ...form, cc })} />
        <TextField label={t('compose.bcc')} value={form.bcc} oninput={(bcc) => onchange({ ...form, bcc })} />
    {/if}

    <TextField
        label={t('compose.subject')}
        value={form.subject}
        oninput={(subject) => onchange({ ...form, subject })}
    />

    <label class="body">
        <span>{t('compose.body')}</span>
        <textarea
            value={form.body}
            oninput={(event) => onchange({ ...form, body: event.currentTarget.value })}
        ></textarea>
    </label>

    {#if error !== null}
        <Notice kind="danger">
            {errorMessage(error)}
            <small>{t('error.details')}: {error.detail}</small>
        </Notice>
    {/if}

    <div class="actions">
        <Button disabled={sending} onclick={ondiscard}>{t('compose.discard')}</Button>
        <Button variant="accent" type="submit" disabled={sending}>
            {sending ? t('compose.sending') : t('compose.send')}
        </Button>
    </div>
</form>

<style>
    .composer {
        display: grid;
        gap: var(--space-3);
    }

    .recipients {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: end;
        gap: var(--space-2);
    }

    .body {
        display: grid;
        gap: var(--space-1);
    }

    .body span {
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    textarea {
        width: 100%;
        height: 240px;
        padding: var(--space-2);
        border: 1px solid var(--field-border);
        border-top-color: var(--field-border-top);
        border-radius: var(--radius-control);
        background: var(--field-background);
        box-shadow: var(--field-inner-shadow);
        resize: vertical;
    }

    textarea:focus-visible {
        outline: none;
        box-shadow: var(--field-inner-shadow), var(--focus-glow);
    }

    small {
        display: block;
        margin-top: var(--space-1);
        color: var(--text-muted);
        overflow-wrap: anywhere;
    }

    .actions {
        display: flex;
        justify-content: flex-end;
        gap: var(--space-2);
    }
</style>

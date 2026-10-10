<script lang="ts">
    import { onMount } from 'svelte'
    import { toMailError } from '$lib/api'
    import type {
        Account,
        AccountSettings,
        AddressError,
        AddressPreset,
        Api,
        Brand,
        Detection,
        MailError,
        Protocol,
        ProviderChoice,
        Security,
        ServerRole,
        SettingsIssue,
        SignIn,
        TypedDomain,
    } from '$lib/api'
    import BrandMark from '$lib/design/BrandMark.svelte'
    import Button from '$lib/design/Button.svelte'
    import Menu from '$lib/design/Menu.svelte'
    import MenuItem from '$lib/design/MenuItem.svelte'
    import Notice from '$lib/design/Notice.svelte'
    import TextField from '$lib/design/TextField.svelte'
    import { errorMessage } from '$lib/features/workspace/workspace'
    import { t } from '$lib/i18n'
    import ServerForm from './ServerForm.svelte'
    import {
        domainOf,
        serverOf,
        switchIncomingProtocol,
        switchSecurity,
        usesSignIn,
        withAuth,
        withServer,
    } from './setup'

    type Props = {
        api: Api
        onadded: (account: Account) => void
        // Present when the setup can be left, i.e. another account already exists.
        oncancel?: () => void
    }

    let { api, onadded, oncancel }: Props = $props()

    const roles: ServerRole[] = ['Incoming', 'Outgoing']

    let email = $state('')
    let addressError = $state<AddressError | null>(null)
    // What the address typed so far says about its provider.
    let typed = $state<TypedDomain>({ type: 'unknown' })
    // A provider whose domain the field holds fixed: only the name before `@` is typed.
    let preset = $state<AddressPreset | null>(null)
    let presets = $state<AddressPreset[]>([])
    // What the field shows: with a preset, the address without its fixed domain.
    const shown = $derived(preset === null ? email : email.slice(0, email.length - preset.domain.length - 1))
    let failed = $state(false)
    let busy = $state(false)

    // null while the user is still on the address step.
    let detection = $state<Detection | null>(null)
    let settings = $state<AccountSettings | null>(null)
    let password = $state('')
    // true when the user asked for manual setup: detection is then only a pre-fill.
    let manual = $state(false)
    let editing = $state(false)
    // Problems found before connecting; any edit clears them.
    let issues = $state<SettingsIssue[]>([])
    let connectError = $state<MailError | null>(null)
    // The sign-in the user is finishing in the browser right now.
    let waiting = $state<SignIn | null>(null)
    // Providers whose account can be picked on their own page, without typing it.
    let choices = $state<ProviderChoice[]>([])

    onMount(() => {
        // Without them the address field is the way in, as always.
        api.signInChoices().then((found) => {
            choices = found
        }).catch(() => undefined)
        // Without them the field takes the whole address, as it always can.
        api.addressPresets().then((found) => {
            presets = found
        }).catch(() => undefined)
    })

    const run = async (action: () => Promise<void>) => {
        busy = true
        failed = false
        try {
            await action()
        } catch {
            failed = true
        } finally {
            busy = false
        }
    }

    const submit = (event: SubmitEvent, action: () => Promise<void>) => {
        event.preventDefault()
        action()
    }

    // How long a typo warning waits, so typing the right ending never flashes it.
    const warningDelay = 350
    let warningTimer: ReturnType<typeof setTimeout> | undefined

    const recognize = async (value: string) => {
        // Only a hint: when the core cannot say, the field works the same without it.
        const unknown: TypedDomain = { type: 'unknown' }
        const answer = await api.recognizeAddress(value).catch(() => unknown)
        // Answers can arrive out of order; only the one for what the field holds counts.
        if (email !== value) {
            return
        }

        clearTimeout(warningTimer)
        // The mark and the clearing of a warning are immediate; only a new warning waits.
        if (answer.type === 'mistyped' && typed.type !== 'mistyped') {
            warningTimer = setTimeout(() => {
                if (email === value) {
                    typed = answer
                }
            }, warningDelay)

            return
        }

        typed = answer
    }

    const typeEmail = (value: string) => {
        email = value
        addressError = null
        recognize(value)
    }

    const typeField = (value: string) => {
        // A whole address typed or pasted is taken as it is.
        if (preset !== null && value.includes('@')) {
            preset = null
        }

        typeEmail(preset === null ? value : value + '@' + preset.domain)
    }

    const localPart = (text: string): string => {
        const at = text.lastIndexOf('@')

        return at === -1 ? text : text.slice(0, at)
    }

    const choosePreset = (next: AddressPreset | null) => {
        if (next === null && preset === null) {
            return
        }

        const local = localPart(shown)
        preset = next
        if (next !== null) {
            typeEmail(local + '@' + next.domain)

            return
        }

        // Back to typing the whole address: what was typed stays, domain included.
        typeEmail(local === '' ? '' : email)
    }

    const useSuggestion = (domain: string) => {
        typeEmail(email.slice(0, email.lastIndexOf('@') + 1) + domain)
    }

    const typePassword = (value: string) => {
        password = value
        connectError = null
    }

    const toggleEditing = () => {
        editing = !editing
    }

    const lookUp = (asManual: boolean) => run(async () => {
        const result = await api.detectAccountSettings(email)
        if (!result.ok) {
            addressError = result.error

            return
        }

        addressError = null
        detection = result.detection
        settings = result.detection.settings
        manual = asManual
        editing = asManual || result.detection.source === 'Guess'
        issues = []
        connectError = null
    })

    const back = () => {
        detection = null
        settings = null
        password = ''
        issues = []
        connectError = null
        failed = false
    }

    const update = (next: AccountSettings) => {
        settings = next
        issues = []
        connectError = null
    }

    const changeProtocol = (current: AccountSettings, found: Detection, protocol: Protocol) => run(async () => {
        update(await switchIncomingProtocol(api, current, found, protocol))
    })

    const changeSecurity = (current: AccountSettings, role: ServerRole, security: Security) => run(async () => {
        update(await switchSecurity(api, current, role, security))
    })

    // Looks for problems that need no network; false when one of them blocks.
    const check = async (current: AccountSettings): Promise<boolean> => {
        connectError = null
        issues = await api.validateAccountSettings(current)
        if (issues.some((issue) => issue.blocking)) {
            editing = true

            return false
        }

        return true
    }

    const explain = (caught: unknown) => {
        connectError = toMailError(caught)
        // The fix is almost always in the settings, so put them in reach.
        if (connectError.kind !== 'Auth' && connectError.kind !== 'SignIn') {
            editing = true
        }
    }

    const connect = (current: AccountSettings) => run(async () => {
        if (!await check(current)) {
            return
        }

        try {
            onadded(await api.addAccount(current, password))
        } catch (caught) {
            explain(caught)
        }
    })

    // Opens the provider's page in the browser and waits until the user is done there.
    const follow = async (begin: () => Promise<SignIn>) => {
        let started: SignIn | null = null
        try {
            started = await begin()
            waiting = started
            await api.openExternal(started.url)
            onadded(await api.finishSignIn(started.id))
        } catch (caught) {
            // A sign-in the user cancelled here needs no explanation.
            if (started === null || waiting !== null) {
                explain(caught)
            }
            if (started !== null) {
                // Nothing may keep listening for a browser that never comes.
                await api.cancelSignIn(started.id).catch(() => undefined)
            }
        } finally {
            waiting = null
        }
    }

    const signIn = (current: AccountSettings) => run(async () => {
        if (!await check(current)) {
            return
        }

        await follow(() => api.beginSignIn(current))
    })

    // The user picks the account on the provider's page; it is added as if typed here.
    const chooseAccount = (brand: Brand) => run(async () => {
        connectError = null
        await follow(() => api.beginChosenSignIn(brand))
    })

    const cancelSignIn = () => {
        const started = waiting
        if (started === null) {
            return
        }

        waiting = null
        api.cancelSignIn(started.id).catch(() => undefined)
    }

    const reopen = (started: SignIn) => {
        api.openExternal(started.url).catch(() => undefined)
    }
</script>

{#snippet mark()}
    {#if typed.type === 'known'}
        <BrandMark brand={typed.brand} label={t('setup.recognized', { provider: typed.provider })} />
    {/if}
{/snippet}

<!-- The provider's mark when it is recognized, an arrow otherwise; either opens the presets. -->
{#snippet providers()}
    <Menu
        label={t('setup.provider')}
        size="small"
        align="end"
        disabled={busy}
        icon={typed.type === 'known' ? undefined : 'chevron'}
        picture={typed.type === 'known' ? mark : undefined}
    >
        {#snippet children(close)}
            <MenuItem
                current={preset === null}
                onclick={() => {
                    close()
                    choosePreset(null)
                }}
            >
                {t('setup.noProvider')}
            </MenuItem>
            {#each presets as option (option.domain)}
                <MenuItem
                    current={preset !== null && preset.domain === option.domain}
                    onclick={() => {
                        close()
                        choosePreset(option)
                    }}
                >
                    {option.provider} <span class="domain">@{option.domain}</span>
                </MenuItem>
            {/each}
        {/snippet}
    </Menu>
{/snippet}

<!-- How a sign-in on the provider's page is going, and why it failed. -->
{#snippet progress()}
    {#if connectError !== null}
        <Notice kind="danger">
            {errorMessage(connectError)}
            <small>{t('error.details')}: {connectError.detail}</small>
        </Notice>
    {/if}
    {#if failed}
        <Notice kind="danger">{t('setup.failed')}</Notice>
    {/if}
    {#if waiting !== null}
        {@const started = waiting}
        <Notice>
            {t('setup.signInWaiting')}
            <Button variant="link" onclick={() => reopen(started)}>{t('setup.signInReopen')}</Button>
        </Notice>
    {/if}
{/snippet}

{#if detection === null || settings === null}
    <form class="stack" novalidate onsubmit={(event) => submit(event, () => lookUp(false))}>
        <div>
            <h1>{t('setup.title')}</h1>
            <p class="muted">{t('setup.subtitle')}</p>
        </div>

        <TextField
            label={t('setup.email')}
            type="email"
            inputmode="email"
            placeholder={preset === null ? t('setup.emailPlaceholder') : ''}
            value={shown}
            suffix={preset === null ? undefined : '@' + preset.domain}
            autofocus
            invalid={addressError !== null}
            trailing={presets.length > 0 ? providers : typed.type === 'known' ? mark : undefined}
            oninput={typeField}
        />

        {#if typed.type === 'mistyped'}
            {@const domain = typed.suggestion}
            <Notice kind="warning">
                {t('setup.mistyped', { provider: typed.provider, domain })}
                <Button variant="link" disabled={busy} onclick={() => useSuggestion(domain)}>
                    {t('setup.useSuggestion', { domain })}
                </Button>
            </Notice>
        {/if}

        {#if addressError !== null}
            <Notice kind="danger">{t(`address.${addressError}`)}</Notice>
        {/if}

        {#if choices.length > 0}
            <div class="choices">
                <p class="divider">{t('setup.orUsing')}</p>
                <div class="providers">
                    {#each choices as choice (choice.brand)}
                        {#snippet picture()}
                            <BrandMark
                                brand={choice.brand}
                                label={t('setup.signIn', { provider: choice.provider })}
                                size={18}
                            />
                        {/snippet}
                        <Button {picture} disabled={busy} onclick={() => chooseAccount(choice.brand)}>
                            {choice.provider}
                        </Button>
                    {/each}
                </div>
            </div>
        {/if}

        {@render progress()}

        <div class="actions">
            {#if waiting !== null}
                <Button variant="link" onclick={cancelSignIn}>{t('setup.signInCancel')}</Button>
            {:else}
                {#if oncancel !== undefined}
                    <Button variant="link" disabled={busy} onclick={oncancel}>{t('setup.cancel')}</Button>
                {/if}
                <!-- Only where there is an easier way in to set aside; an unknown domain opens the form anyway. -->
                {#if typed.type === 'known'}
                    <Button variant="link" disabled={busy} onclick={() => lookUp(true)}>{t('setup.manual')}</Button>
                {/if}
            {/if}
            <span class="spacer"></span>
            <Button variant="accent" type="submit" disabled={busy}>{t('setup.continue')}</Button>
        </div>
    </form>
{:else}
    {@const current = settings}
    {@const found = detection}
    {@const signingIn = usesSignIn(current)}
    <form
        class="stack"
        novalidate
        onsubmit={(event) => submit(event, () => signingIn ? signIn(current) : connect(current))}
    >
        <div>
            {#if manual}
                <h1>{t('setup.manualTitle')}</h1>
                <p class="muted">{t('setup.manualHint')}</p>
            {:else if found.provider !== null}
                <h1>{t('setup.found', { provider: found.provider })}</h1>
                <p class="muted">{t('setup.foundHint')}</p>
            {:else}
                <h1>{t('setup.guess', { domain: domainOf(current.email) })}</h1>
                <p class="muted">{t('setup.guessHint')}</p>
            {/if}
        </div>

        {#if signingIn && !found.signInAvailable}
            <Notice kind="warning">{t('setup.oauthUnavailable')}</Notice>
        {:else if signingIn}
            <Notice>{t('setup.signInHint')}</Notice>
        {:else if found.passwordKind === 'AppPassword'}
            <Notice>{t('setup.appPassword')}</Notice>
        {/if}
        {#if current.incoming.protocol === 'Pop3'}
            <Notice kind="warning">{t('setup.pop3Unavailable')}</Notice>
        {/if}

        <TextField
            label={t('setup.displayName')}
            placeholder={t('setup.displayNamePlaceholder')}
            value={current.displayName}
            oninput={(displayName) => update({ ...current, displayName })}
        />

        {#if signingIn}
            <!-- The manual way stays one step away (AGENTS.md 2.2). -->
            <div>
                <Button variant="link" disabled={busy} onclick={() => update(withAuth(current, 'Password'))}>
                    {t('setup.usePassword')}
                </Button>
            </div>
        {:else}
            <div>
                <TextField
                    label={t('setup.password')}
                    type="password"
                    value={password}
                    autofocus
                    invalid={connectError !== null && connectError.kind === 'Auth'}
                    oninput={typePassword}
                />
                <p class="hint">{t('setup.passwordHint')}</p>
                {#if found.signInAvailable}
                    <Button variant="link" disabled={busy} onclick={() => update(withAuth(current, 'OAuth2'))}>
                        {t('setup.useSignIn')}
                    </Button>
                {/if}
            </div>
        {/if}

        {#if editing}
            {#each roles as role (role)}
                <ServerForm
                    {role}
                    server={serverOf(current, role)}
                    {issues}
                    onchange={(server) => update(withAuth(withServer(current, role, server), server.auth))}
                    onprotocol={(protocol) => changeProtocol(current, found, protocol)}
                    onsecurity={(security) => changeSecurity(current, role, security)}
                />
            {/each}
        {:else}
            <dl class="summary">
                {#each roles as role (role)}
                    {@const server = serverOf(current, role)}
                    <dt>{t(`role.${role}`)}</dt>
                    <dd>{t(`protocol.${server.protocol}`)} · {server.host}:{server.port} · {t(`security.${server.security}`)}</dd>
                {/each}
            </dl>
        {/if}

        {#each issues as issue (issue.server + issue.kind)}
            <Notice kind={issue.blocking ? 'danger' : 'warning'}>
                <strong>{t(`role.${issue.server}`)}:</strong> {t(`issue.${issue.kind}`)}
            </Notice>
        {/each}
        {@render progress()}

        <div class="actions">
            {#if waiting !== null}
                <Button variant="link" onclick={cancelSignIn}>{t('setup.signInCancel')}</Button>
            {:else}
                <Button variant="link" disabled={busy} onclick={back}>{t('setup.back')}</Button>
            {/if}
            <span class="spacer"></span>
            <Button disabled={busy} onclick={toggleEditing}>
                {editing ? t('setup.hide') : t('setup.edit')}
            </Button>
            <Button variant="accent" type="submit" disabled={busy}>
                {#if waiting !== null}
                    {t('setup.waiting')}
                {:else if busy}
                    {t('setup.connecting')}
                {:else if !signingIn}
                    {t('setup.connect')}
                {:else if found.provider !== null}
                    {t('setup.signIn', { provider: found.provider })}
                {:else}
                    {t('setup.signInGeneric')}
                {/if}
            </Button>
        </div>
    </form>
{/if}

<style>
    .stack {
        display: grid;
        gap: var(--space-4);
    }

    h1 {
        font-size: var(--font-size-title);
        font-weight: 400;
        color: var(--text-link);
    }

    .muted {
        margin-top: var(--space-1);
        color: var(--text-muted);
    }

    .hint {
        margin-top: var(--space-1);
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    .summary {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        gap: var(--space-2) var(--space-4);
        margin: 0;
        padding: var(--space-3);
        border: 1px solid var(--surface-border);
        border-radius: var(--radius-panel);
        background: var(--surface-sunken);
        user-select: text;
        -webkit-user-select: text;
    }

    dt {
        color: var(--text-muted);
    }

    dd {
        margin: 0;
        overflow-wrap: anywhere;
    }

    small {
        display: block;
        margin-top: var(--space-1);
        color: var(--text-muted);
        overflow-wrap: anywhere;
    }

    .domain {
        color: var(--text-muted);
    }

    .choices {
        display: grid;
        gap: var(--space-2);
    }

    /* A rule on each side of the words. */
    .divider {
        display: flex;
        align-items: center;
        gap: var(--space-3);
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    .divider::before,
    .divider::after {
        content: '';
        flex: 1;
        border-top: 1px solid var(--surface-border);
    }

    .providers {
        display: flex;
        flex-wrap: wrap;
        justify-content: center;
        gap: var(--space-2);
    }

    .actions {
        display: flex;
        align-items: center;
        gap: var(--space-2);
    }

    .spacer {
        flex: 1;
    }
</style>

<script lang="ts">
    import type { NotificationDuration, NotificationIcon, NotificationSettings } from '$lib/api'
    import favicon from '$lib/assets/favicon.svg'
    import { iconFromFile } from '$lib/design/avatarImage'
    import Avatar from '$lib/design/Avatar.svelte'
    import Button from '$lib/design/Button.svelte'
    import CheckField from '$lib/design/CheckField.svelte'
    import Notice from '$lib/design/Notice.svelte'
    import SelectField from '$lib/design/SelectField.svelte'
    import TextField from '$lib/design/TextField.svelte'
    import { t } from '$lib/i18n'
    import {
        bodyPresets,
        fieldToken,
        newMailText,
        presetOf,
        sampleMail,
        templateFields,
        titlePresets,
        type TemplateField,
    } from './notification'

    type Props = {
        settings: NotificationSettings
        // Every change is applied at once; there is nothing to save.
        onchange: (settings: NotificationSettings) => void
        // Puts every notification setting back to what a fresh install has.
        onreset: () => void
    }

    let { settings, onchange, onreset }: Props = $props()

    // The deep part stays folded away until asked for.
    let customizing = $state(false)
    // Set when "your own text" is picked for a text that still equals a
    // preset, so its field shows before anything was typed.
    let ownTitle = $state(false)
    let ownBody = $state(false)
    // What is typed into a number field while it is not a usable number yet.
    let secondsDraft = $state<string | null>(null)
    let lengthDraft = $state<string | null>(null)
    let pictureUnreadable = $state(false)
    let fileInput = $state<HTMLInputElement | null>(null)

    const secondsRange = { low: 1, high: 3600 }
    const lengthRange = { low: 10, high: 500 }

    let titleChoice = $derived(ownTitle ? 'custom' : presetOf(titlePresets, settings.title))
    let bodyChoice = $derived(ownBody ? 'custom' : presetOf(bodyPresets, settings.body))
    let showsPreview = $derived(settings.title.includes('{preview}') || settings.body.includes('{preview}'))
    let sample = $derived(newMailText(sampleMail(), settings))
    let sampleSender = $derived(sampleMail().messages[0]?.from ?? null)

    const change = (part: Partial<NotificationSettings>) => {
        onchange({ ...settings, ...part })
    }

    const titleOptions = [
        ...titlePresets.map((preset) => ({ value: preset.id, label: t(`notifyTitle.${preset.id}`) })),
        { value: 'custom', label: t('notifyPreset.custom') },
    ]
    const bodyOptions = [
        ...bodyPresets.map((preset) => ({ value: preset.id, label: t(`notifyBody.${preset.id}`) })),
        { value: 'custom', label: t('notifyPreset.custom') },
    ]
    const iconChoices: NotificationIcon[] = ['Sender', 'App', 'Custom', 'None']
    const iconOptions = iconChoices.map((value) => ({ value, label: t(`notifyIcon.${value}`) }))
    const durationChoices: NotificationDuration[] = ['System', 'Custom', 'UntilDismissed']
    const durationOptions = durationChoices.map((value) => ({ value, label: t(`notifyDuration.${value}`) }))

    const chooseTitle = (choice: string) => {
        const preset = titlePresets.find((candidate) => candidate.id === choice)
        ownTitle = preset === undefined
        if (preset !== undefined) {
            change({ title: preset.template })
        }
    }

    const chooseBody = (choice: string) => {
        const preset = bodyPresets.find((candidate) => candidate.id === choice)
        ownBody = preset === undefined
        if (preset !== undefined) {
            change({ body: preset.template })
        }
    }

    // A field goes at the end, on the same line, with a space before it.
    const withField = (template: string, field: TemplateField): string => {
        if (template === '' || template.endsWith(' ') || template.endsWith('\n')) {
            return template + fieldToken(field)
        }

        return `${template} ${fieldToken(field)}`
    }

    // A whole number within the range, or null.
    const numberIn = (text: string, range: { low: number, high: number }): number | null => {
        if (!/^\d+$/.test(text.trim())) {
            return null
        }
        const value = Number(text.trim())

        return value >= range.low && value <= range.high ? value : null
    }

    const typeSeconds = (text: string) => {
        const seconds = numberIn(text, secondsRange)
        secondsDraft = seconds === null ? text : null
        if (seconds !== null) {
            change({ seconds })
        }
    }

    const typeLength = (text: string) => {
        const previewLength = numberIn(text, lengthRange)
        lengthDraft = previewLength === null ? text : null
        if (previewLength !== null) {
            change({ previewLength })
        }
    }

    const choosePicture = async (files: FileList | null) => {
        const file = files?.[0]
        if (file === undefined) {
            return
        }
        const customIcon = await iconFromFile(file)
        pictureUnreadable = customIcon === null
        if (customIcon !== null) {
            change({ icon: 'Custom', customIcon })
        }
    }

    const reset = () => {
        ownTitle = false
        ownBody = false
        secondsDraft = null
        lengthDraft = null
        pictureUnreadable = false
        onreset()
    }

    const rangeHint = (range: { low: number, high: number }): string => {
        return t('settings.range', { low: String(range.low), high: String(range.high) })
    }
</script>

{#snippet fieldButtons(template: string, apply: (template: string) => void)}
    <div class="fields" role="group" aria-label={t('settings.notifyInsert')}>
        <span>{t('settings.notifyInsert')}:</span>
        {#each templateFields as field (field)}
            <Button
                label={t('settings.notifyInsertField', { field: t(`notifyField.${field}`) })}
                onclick={() => apply(withField(template, field))}
            >{t(`notifyField.${field}`)}</Button>
        {/each}
    </div>
{/snippet}

<div class="options">
    <CheckField
        label={t('settings.notifyNewMail')}
        hint={t('settings.notifyNewMailHint')}
        checked={settings.enabled}
        onchange={(enabled) => change({ enabled })}
    />

    {#if settings.enabled}
        <CheckField
            label={t('settings.notifySound')}
            checked={settings.sound}
            onchange={(sound) => change({ sound })}
        />

        <figure>
            <figcaption>{t('settings.notifySample')}</figcaption>
            <div class="sample" data-testid="notification-sample">
                {#if settings.icon === 'Sender' && sampleSender !== null}
                    <Avatar name={sampleSender.name ?? sampleSender.email} seed={sampleSender.email} size="large" />
                {:else if settings.icon === 'App' || (settings.icon === 'Custom' && settings.customIcon === null)}
                    <img class="picture" src={favicon} alt="" />
                {:else if settings.icon === 'Custom' && settings.customIcon !== null}
                    <img class="picture" src={settings.customIcon} alt="" />
                {/if}
                {#if sample !== null}
                    <div class="words">
                        <strong>{sample.title}</strong>
                        {#if sample.body !== ''}
                            <span class="body">{sample.body}</span>
                        {/if}
                    </div>
                {/if}
            </div>
        </figure>

        <div class="row">
            <Button
                variant="link"
                icon="chevron"
                expanded={customizing}
                onclick={() => customizing = !customizing}
            >{t('settings.notifyCustomize')}</Button>
        </div>

        {#if customizing}
            <div class="deep">
                <SelectField
                    label={t('settings.notifyTitle')}
                    value={titleChoice}
                    options={titleOptions}
                    onchange={chooseTitle}
                />
                {#if titleChoice === 'custom'}
                    <TextField
                        label={t('settings.notifyTemplate')}
                        value={settings.title}
                        hint={t('settings.notifyTemplateHint')}
                        oninput={(title) => change({ title })}
                    />
                    {@render fieldButtons(settings.title, (title) => change({ title }))}
                {/if}

                <SelectField
                    label={t('settings.notifyBody')}
                    value={bodyChoice}
                    options={bodyOptions}
                    onchange={chooseBody}
                />
                {#if bodyChoice === 'custom'}
                    <TextField
                        label={t('settings.notifyTemplate')}
                        value={settings.body}
                        multiline
                        hint={t('settings.notifyTemplateHint')}
                        oninput={(body) => change({ body })}
                    />
                    {@render fieldButtons(settings.body, (body) => change({ body }))}
                {/if}

                {#if showsPreview}
                    <TextField
                        label={t('settings.notifyPreviewLength')}
                        value={lengthDraft ?? String(settings.previewLength)}
                        inputmode="numeric"
                        invalid={lengthDraft !== null}
                        hint={rangeHint(lengthRange)}
                        oninput={typeLength}
                    />
                {/if}

                <SelectField
                    label={t('settings.notifyIcon')}
                    value={settings.icon}
                    options={iconOptions}
                    onchange={(icon) => change({ icon })}
                />
                {#if settings.icon === 'Custom'}
                    <div class="row">
                        {#if settings.customIcon !== null}
                            <img class="picture" src={settings.customIcon} alt="" />
                        {/if}
                        <Button icon="image" onclick={() => fileInput?.click()}>{t('settings.notifyChoosePicture')}</Button>
                        <input
                            bind:this={fileInput}
                            type="file"
                            accept="image/*"
                            hidden
                            data-testid="notification-picture"
                            onchange={(event) => choosePicture(event.currentTarget.files)}
                        />
                    </div>
                    {#if pictureUnreadable}
                        <Notice kind="danger">{t('settings.notifyPictureUnreadable')}</Notice>
                    {/if}
                {/if}

                <SelectField
                    label={t('settings.notifyDuration')}
                    value={settings.duration}
                    options={durationOptions}
                    onchange={(duration) => change({ duration })}
                />
                {#if settings.duration === 'Custom'}
                    <TextField
                        label={t('settings.notifySeconds')}
                        value={secondsDraft ?? String(settings.seconds)}
                        inputmode="numeric"
                        invalid={secondsDraft !== null}
                        hint={rangeHint(secondsRange)}
                        oninput={typeSeconds}
                    />
                {/if}
                {#if settings.duration !== 'System'}
                    <small>{t('settings.notifyDurationHint')}</small>
                {/if}

                <CheckField
                    label={t('settings.notifyWhileFocused')}
                    hint={t('settings.notifyWhileFocusedHint')}
                    checked={settings.whileFocused}
                    onchange={(whileFocused) => change({ whileFocused })}
                />

                <div class="row">
                    <Button variant="link" onclick={reset}>{t('settings.notifyReset')}</Button>
                </div>
            </div>
        {/if}
    {/if}
</div>

<style>
    .options,
    .deep {
        display: grid;
        gap: var(--space-3);
    }

    .deep {
        padding-left: var(--space-4);
        border-left: 1px solid var(--row-separator);
    }

    figure {
        display: grid;
        gap: var(--space-1);
        margin: 0;
    }

    figcaption,
    small,
    .fields span {
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    /* Drawn like a notification: a raised card with a picture and two lines. */
    .sample {
        display: flex;
        align-items: flex-start;
        gap: var(--space-3);
        padding: var(--space-3);
        border: 1px solid var(--menu-border);
        border-radius: var(--radius-panel);
        background: var(--menu-background);
        box-shadow: var(--menu-shadow);
    }

    .words {
        display: grid;
        gap: var(--space-1);
        min-width: 0;
    }

    .body {
        white-space: pre-line;
        overflow-wrap: anywhere;
        color: var(--text-muted);
    }

    .picture {
        flex: none;
        width: 38px;
        height: 38px;
        border-radius: var(--radius-control);
        object-fit: cover;
    }

    .row,
    .fields {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: var(--space-2);
    }

    .fields {
        gap: var(--space-1);
    }
</style>

// What a new-mail notification says and shows, from the user's notification
// settings. The shell only displays the result (`api.notify`).

import type { MailEvent, NewMailItem, NotificationIconRequest, NotificationRequest, NotificationSettings } from '$lib/api'
import { avatarImage } from '$lib/design/avatarImage'
import { t } from '$lib/i18n'

export type NewMail = Extract<MailEvent, { type: 'newMail' }>

// The words a title or text template may contain, each written in braces.
export const templateFields = [
    'sender',
    'address',
    'subject',
    'preview',
    'account',
] as const

export type TemplateField = (typeof templateFields)[number]

const isField = (name: string): name is TemplateField => {
    return templateFields.some((field) => field === name)
}

export const fieldToken = (field: TemplateField): string => {
    return `{${field}}`
}

// Fills the fields of a template. A line that only held fields which turned
// out empty (a message without text) is left out instead of staying blank.
export const fillTemplate = (template: string, values: Record<TemplateField, string>): string => {
    return template
        .split('\n')
        .map((line) => line.replace(/\{(\w+)\}/g, (whole, name: string) => isField(name) ? values[name] : whole).trim())
        .filter((line) => line !== '')
        .join('\n')
}

// At most `length` characters, ending with an ellipsis when something was cut;
// a word is not cut in half when a space is near.
export const shorten = (text: string, length: number): string => {
    const characters = Array.from(text)
    if (characters.length <= length) {
        return text
    }

    const cut = characters.slice(0, length).join('')
    const space = cut.lastIndexOf(' ')
    const end = space >= length * 0.7 ? cut.slice(0, space) : cut

    // 'spring,…' reads as a mistake; 'spring…' does not.
    return end.trimEnd().replace(/[,;:]+$/, '') + '…'
}

const senderOf = (item: NewMailItem): string => {
    if (item.from === null) {
        return t('list.unknownSender')
    }

    return item.from.name ?? item.from.email
}

const subjectOf = (item: NewMailItem): string => {
    return item.subject === '' ? t('list.noSubject') : item.subject
}

export const templateValues = (item: NewMailItem, account: string, previewLength: number): Record<TemplateField, string> => {
    return {
        sender: senderOf(item),
        address: item.from?.email ?? '',
        subject: subjectOf(item),
        preview: item.preview === null ? '' : shorten(item.preview, previewLength),
        account,
    }
}

// Notifications name this many messages; the rest is counted.
const namedMessages = 3

export type NotificationText = {
    title: string
    body: string
}

// One message is told as the user's templates say; several at once become
// one short list, so a burst of mail is not a burst of notifications.
export const newMailText = (event: NewMail, settings: NotificationSettings): NotificationText | null => {
    const first = event.messages[0]
    if (first === undefined) {
        return null
    }

    if (event.messages.length === 1) {
        const values = templateValues(first, event.account, settings.previewLength)
        const title = fillTemplate(settings.title, values)

        return {
            // Every notification service needs some title.
            title: title === '' ? t('app.name') : title,
            body: fillTemplate(settings.body, values),
        }
    }

    const lines = event.messages
        .slice(0, namedMessages)
        .map((item) => `${senderOf(item)}: ${subjectOf(item)}`)
    const rest = event.messages.length - lines.length
    if (rest > 0) {
        lines.push(t('notify.more', { count: String(rest) }))
    }

    return {
        title: t('notify.many', { count: String(event.messages.length) }),
        body: lines.join('\n'),
    }
}

// Where a sender's own picture comes from, when there is one.
export type PictureSource = (email: string) => Promise<string | null>

export const notificationIcon = async (
    event: NewMail,
    settings: NotificationSettings,
    pictureOf: PictureSource,
): Promise<NotificationIconRequest> => {
    if (settings.icon === 'None') {
        return { kind: 'none' }
    }
    if (settings.icon === 'Custom') {
        return settings.customIcon === null ? { kind: 'app' } : { kind: 'image', url: settings.customIcon }
    }
    if (settings.icon === 'App') {
        return { kind: 'app' }
    }

    // One face only when all of it is from one sender.
    const first = event.messages[0]?.from ?? null
    const senders = new Set(event.messages.map((item) => item.from?.email.toLowerCase()))
    if (first === null || senders.size !== 1) {
        return { kind: 'app' }
    }

    // A picture that cannot be had leaves the initial, never no notification.
    const picture = await pictureOf(first.email).catch(() => null)
    const url = await avatarImage(first.name ?? first.email, first.email, picture)

    return url === null ? { kind: 'app' } : { kind: 'image', url }
}

export const newMailNotification = async (
    event: NewMail,
    settings: NotificationSettings,
    pictureOf: PictureSource,
): Promise<NotificationRequest | null> => {
    const text = newMailText(event, settings)
    const first = event.messages[0]
    if (text === null || first === undefined) {
        return null
    }

    return {
        ...text,
        icon: await notificationIcon(event, settings, pictureOf),
        sound: settings.sound,
        duration: settings.duration,
        seconds: settings.seconds,
        whileFocused: settings.whileFocused,
        target: { accountId: event.accountId, folderId: event.folderId, messageId: first.id },
    }
}

// Ready-made templates, so the common choices are one click.
export const titlePresets = [
    { id: 'sender', template: '{sender}' },
    { id: 'subject', template: '{subject}' },
    { id: 'address', template: '{address}' },
    { id: 'account', template: '{account}' },
] as const

export const bodyPresets = [
    { id: 'subjectPreview', template: '{subject}\n{preview}' },
    { id: 'subject', template: '{subject}' },
    { id: 'preview', template: '{preview}' },
    { id: 'senderPreview', template: '{sender}: {preview}' },
    { id: 'nothing', template: '' },
] as const

type Preset<Id extends string> = { readonly id: Id, readonly template: string }

// The preset a template is, or 'custom' for one the user wrote.
export const presetOf = <Id extends string>(presets: readonly Preset<Id>[], template: string): Id | 'custom' => {
    return presets.find((preset) => preset.template === template)?.id ?? 'custom'
}

// A made-up message the settings show their notification with.
export const sampleMail = (): NewMail => {
    return {
        type: 'newMail',
        accountId: 0,
        folderId: 0,
        account: t('notify.sampleAccount'),
        messages: [
            {
                id: 0,
                subject: t('notify.sampleSubject'),
                from: { name: t('notify.sampleSender'), email: t('notify.sampleAddress') },
                preview: t('notify.samplePreview'),
            },
        ],
    }
}

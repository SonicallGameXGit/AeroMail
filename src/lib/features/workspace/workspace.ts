// Presentation helpers of the workspace. No mail rules live here.

import type { Account, Contact, Draft, Folder, FolderRole, MailError, MessageSummary, OutgoingMessage } from '$lib/api'
import type { IconName } from '$lib/design/icons'
import { t } from '$lib/i18n'

export const folderLabel = (folder: Folder): string => {
    if (folder.role === 'Other') {
        return folder.name
    }

    // Template literal keeps the key type-checked against the dictionary.
    return t(`folder.${folder.role}`)
}

// Which picture stands for a folder, so it can be found without reading.
export const folderIcon = (folder: Folder): IconName => {
    const icons: Record<FolderRole, IconName> = {
        Inbox: 'inbox',
        Flagged: 'flag',
        Drafts: 'drafts',
        Sent: 'sent',
        Archive: 'archive',
        Junk: 'junk',
        Trash: 'trash',
        All: 'all',
        Other: 'folder',
    }

    return icons[folder.role]
}

// The order well-known folders are shown in, whatever the server's order is.
const mainOrder: FolderRole[] = [
    'Inbox',
    'Flagged',
    'Drafts',
    'Sent',
    'Archive',
    'Junk',
    'Trash',
    'All',
]

export type FolderEntry = {
    folder: Folder
    // How many of the folder's parents are shown above it.
    indent: number
}

export type FolderGroups = {
    // Well-known folders, in the same order for every account.
    main: Folder[]
    // The user's own folders, in the server's order.
    custom: FolderEntry[]
}

// Splits an account's folders (in the server's tree order) into the two
// groups of the sidebar. A folder is indented only under parents that are
// shown with it: '[Gmail]/Important' sits at the top level, because
// '[Gmail]' itself is never shown.
export const groupFolders = (folders: Folder[]): FolderGroups => {
    const shown = folders.filter((folder) => folder.selectable)
    const main = mainOrder.flatMap((role) => shown.filter((folder) => folder.role === role))

    const custom: FolderEntry[] = []
    // For each level above the current folder: is that parent shown in `custom`?
    let parents: boolean[] = []
    for (const folder of folders) {
        const listed = folder.selectable && folder.role === 'Other'
        parents = parents.slice(0, folder.depth)
        if (listed) {
            custom.push({ folder, indent: parents.filter((parent) => parent).length })
        }
        parents[folder.depth] = listed
    }

    return { main, custom }
}

export const inboxUnread = (folders: Folder[]): number => {
    return folders
        .filter((folder) => folder.role === 'Inbox')
        .reduce((sum, folder) => sum + folder.unread, 0)
}

// What an account is called in the window: the name its owner typed, or the address.
export const accountLabel = (account: Account): string => {
    if (account.settings.displayName.trim() === '') {
        return account.settings.email
    }

    return account.settings.displayName
}

export const contactLabel = (contact: Contact): string => {
    if (contact.name === null) {
        return contact.email
    }

    return contact.name
}

export const contactFull = (contact: Contact): string => {
    if (contact.name === null) {
        return contact.email
    }

    return `${contact.name} <${contact.email}>`
}

// Sender pictures that were looked up, by `pictureKey`.
export type Pictures = Record<string, string>

// Addresses differ only by case for the same sender.
export const pictureKey = (email: string): string => {
    return email.trim().toLowerCase()
}

export const senderPicture = (pictures: Pictures, message: MessageSummary): string | null => {
    const first = message.from[0]
    if (first === undefined) {
        return null
    }

    return pictures[pictureKey(first.email)] ?? null
}

export const senderLabel = (message: MessageSummary): string => {
    const first = message.from[0]
    if (first === undefined) {
        return t('list.unknownSender')
    }

    return contactLabel(first)
}

export const subjectLabel = (message: MessageSummary): string => {
    if (message.subject === '') {
        return t('list.noSubject')
    }

    return message.subject
}

const sameDay = (a: Date, b: Date): boolean => {
    return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate()
}

// Time for today's mail, day and month this year, full date before that.
export const shortDate = (unixSeconds: number, now: Date, locale?: string): string => {
    const date = new Date(unixSeconds * 1000)
    if (sameDay(date, now)) {
        return new Intl.DateTimeFormat(locale, { hour: '2-digit', minute: '2-digit' }).format(date)
    }
    if (date.getFullYear() === now.getFullYear()) {
        return new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short' }).format(date)
    }

    return new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short', year: 'numeric' }).format(date)
}

export const fullDate = (unixSeconds: number, locale?: string): string => {
    return new Intl.DateTimeFormat(locale, { dateStyle: 'full', timeStyle: 'short' }).format(new Date(unixSeconds * 1000))
}

export const sizeLabel = (bytes: number): string => {
    if (bytes < 1024) {
        return bytes + ' B'
    }
    if (bytes < 1024 * 1024) {
        return Math.round(bytes / 1024) + ' KB'
    }

    return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

export const splitAddresses = (text: string): string[] => {
    return text
        .split(/[,;\n]/)
        .map((address) => address.trim())
        .filter((address) => address !== '')
}

export type ComposeForm = {
    accountId: number
    to: string
    cc: string
    bcc: string
    subject: string
    body: string
    inReplyTo: string | null
    references: string[]
}

export const emptyForm = (accountId: number): ComposeForm => {
    return {
        accountId,
        to: '',
        cc: '',
        bcc: '',
        subject: '',
        body: '',
        inReplyTo: null,
        references: [],
    }
}

export const formFromDraft = (accountId: number, draft: Draft): ComposeForm => {
    return {
        accountId,
        to: draft.to.join(', '),
        cc: draft.cc.join(', '),
        bcc: '',
        subject: draft.subject,
        body: draft.body,
        inReplyTo: draft.inReplyTo,
        references: draft.references,
    }
}

export const toOutgoing = (form: ComposeForm): OutgoingMessage => {
    return {
        accountId: form.accountId,
        to: splitAddresses(form.to),
        cc: splitAddresses(form.cc),
        bcc: splitAddresses(form.bcc),
        subject: form.subject,
        body: form.body,
        inReplyTo: form.inReplyTo,
        references: form.references,
    }
}

export const errorMessage = (error: MailError): string => {
    const message = t(`error.${error.kind}`)
    if (error.server === null) {
        return message
    }

    return `${t(`role.${error.server}`)}: ${message}`
}

// The document shown inside the isolated message frame. The content policy
// is the second line of defence after sanitising in the core: nothing but
// inline styles and images, and remote images only when allowed.
export const frameDocument = (html: string, allowRemote: boolean): string => {
    const images = allowRemote ? 'data: http: https:' : 'data:'
    const policy = `default-src 'none'; style-src 'unsafe-inline'; img-src ${images}`
    // Only defaults for mail that brings no styling of its own: the message's
    // style sheet comes later and wins. Nothing here may change how a laid-out
    // message is measured (no size limits on images, and `break-word` rather
    // than `anywhere`, which would let table columns collapse).
    const style = 'body{margin:12px;font-family:\'Segoe UI\',\'Noto Sans\',sans-serif;font-size:13px;'
        + 'color:#1b1f24;background:#ffffff;overflow-wrap:break-word}'
        + 'a{color:#0b5cad}blockquote{margin:0 0 0 8px;padding-left:8px;border-left:2px solid #a9b9cc}'

    return '<!doctype html><html><head><meta charset="utf-8">'
        + `<meta http-equiv="Content-Security-Policy" content="${policy}">`
        + `<style>${style}</style></head><body>${html}</body></html>`
}

// Only these may leave the app through a link in a message.
export const isSafeLink = (url: string): boolean => {
    return /^(https?:|mailto:)/i.test(url)
}

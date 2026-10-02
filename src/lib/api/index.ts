// The only place in the UI that talks to the Rust core (AGENTS.md 5.2).
// Components receive an `Api` and never reach for the shell themselves, so the
// same UI can later run against another transport or a fake in tests.

import type { DesktopBridge, Reply } from './bridge'
import type { Account } from './generated/Account'
import type { AccountSettings } from './generated/AccountSettings'
import type { AddressError } from './generated/AddressError'
import type { AppSettings } from './generated/AppSettings'
import type { Attachment } from './generated/Attachment'
import type { Detection } from './generated/Detection'
import type { Draft } from './generated/Draft'
import type { ErrorKind } from './generated/ErrorKind'
import type { Folder } from './generated/Folder'
import type { MailError } from './generated/MailError'
import type { MessageContent } from './generated/MessageContent'
import type { MessageSummary } from './generated/MessageSummary'
import type { OutgoingMessage } from './generated/OutgoingMessage'
import type { Protocol } from './generated/Protocol'
import type { ReplyMode } from './generated/ReplyMode'
import type { Security } from './generated/Security'
import type { SettingsIssue } from './generated/SettingsIssue'
import type { SignIn } from './generated/SignIn'

export type { Account } from './generated/Account'
export type { AccountSettings } from './generated/AccountSettings'
export type { AddressError } from './generated/AddressError'
export type { AppSettings } from './generated/AppSettings'
export type { Attachment } from './generated/Attachment'
export type { AuthMethod } from './generated/AuthMethod'
export type { Contact } from './generated/Contact'
export type { Detection } from './generated/Detection'
export type { DetectionSource } from './generated/DetectionSource'
export type { Draft } from './generated/Draft'
export type { ErrorKind } from './generated/ErrorKind'
export type { Folder } from './generated/Folder'
export type { FolderRole } from './generated/FolderRole'
export type { IssueKind } from './generated/IssueKind'
export type { MailError } from './generated/MailError'
export type { MessageContent } from './generated/MessageContent'
export type { MessageFlags } from './generated/MessageFlags'
export type { MessageSummary } from './generated/MessageSummary'
export type { OutgoingMessage } from './generated/OutgoingMessage'
export type { PasswordKind } from './generated/PasswordKind'
export type { Protocol } from './generated/Protocol'
export type { ReplyMode } from './generated/ReplyMode'
export type { Security } from './generated/Security'
export type { ServerRole } from './generated/ServerRole'
export type { ServerSettings } from './generated/ServerSettings'
export type { SettingsIssue } from './generated/SettingsIssue'
export type { SignIn } from './generated/SignIn'

export type DetectResult =
    | { ok: true, detection: Detection }
    | { ok: false, error: AddressError }

// Every method that can fail rejects with a `MailError`; `toMailError` turns
// whatever was caught back into one.
export type Api = {
    detectAccountSettings: (email: string) => Promise<DetectResult>
    validateAccountSettings: (settings: AccountSettings) => Promise<SettingsIssue[]>
    defaultPort: (protocol: Protocol, security: Security) => Promise<number>

    listAccounts: () => Promise<Account[]>
    addAccount: (settings: AccountSettings, password: string) => Promise<Account>
    // Adding an account on the provider's own page instead of with a password:
    // `beginSignIn` gives the page to open in the browser, `finishSignIn`
    // resolves once the user is done there, or rejects when it is cancelled.
    beginSignIn: (settings: AccountSettings) => Promise<SignIn>
    finishSignIn: (signInId: number) => Promise<Account>
    cancelSignIn: (signInId: number) => Promise<void>
    removeAccount: (accountId: number) => Promise<void>

    listFolders: (accountId: number) => Promise<Folder[]>
    refreshFolders: (accountId: number) => Promise<Folder[]>

    listMessages: (folderId: number) => Promise<MessageSummary[]>
    syncFolder: (folderId: number, grow: boolean) => Promise<MessageSummary[]>
    openMessage: (messageId: number, allowRemote: boolean) => Promise<MessageContent>
    setSeen: (messageIds: number[], seen: boolean) => Promise<void>
    setFlagged: (messageIds: number[], flagged: boolean) => Promise<void>
    deleteMessages: (messageIds: number[]) => Promise<void>
    moveMessages: (messageIds: number[], folderId: number) => Promise<void>

    replyDraft: (messageId: number, mode: ReplyMode) => Promise<Draft>
    sendMessage: (message: OutgoingMessage) => Promise<void>

    // Asks where to save; resolves to false when the user cancels.
    saveAttachment: (messageId: number, attachment: Attachment) => Promise<boolean>
    openExternal: (url: string) => Promise<void>

    // The picture a sender's domain publishes, as an address for an image;
    // null when it has none or when pictures are turned off.
    senderPicture: (email: string) => Promise<string | null>

    settings: () => Promise<AppSettings>
    setSettings: (settings: AppSettings) => Promise<void>
}

const addressErrors: readonly string[] = [
    'Empty',
    'MissingAt',
    'MultipleAt',
    'EmptyLocalPart',
    'InvalidLocalPart',
    'InvalidDomain',
] satisfies AddressError[]

export const isAddressError = (value: unknown): value is AddressError => {
    return typeof value === 'string' && addressErrors.includes(value)
}

const errorKinds: readonly string[] = [
    'Connect',
    'Tls',
    'Auth',
    'SignIn',
    'Unsupported',
    'Protocol',
    'Invalid',
    'Keychain',
    'Storage',
    'NotFound',
] satisfies ErrorKind[]

const isErrorKind = (value: unknown): value is ErrorKind => {
    return typeof value === 'string' && errorKinds.includes(value)
}

const isMailError = (value: unknown): value is MailError => {
    if (typeof value !== 'object' || value === null) {
        return false
    }
    if (!('kind' in value) || !('server' in value) || !('detail' in value)) {
        return false
    }

    const serverIsValid = value.server === null || value.server === 'Incoming' || value.server === 'Outgoing'

    return isErrorKind(value.kind) && serverIsValid && typeof value.detail === 'string'
}

export const toMailError = (error: unknown): MailError => {
    if (isMailError(error)) {
        return error
    }

    const detail = error instanceof Error ? error.message : String(error)

    return { kind: 'Protocol', server: null, detail }
}

const bridge = (): DesktopBridge => {
    if (window.mailclient === undefined) {
        const error: MailError = {
            kind: 'Unsupported',
            server: null,
            detail: 'the mail core is only reachable inside the desktop app',
        }

        throw error
    }

    return window.mailclient
}

const unwrap = <T>(reply: Reply<T>): T => {
    if (!reply.ok) {
        throw reply.error
    }

    return reply.value
}

// Components pass their reactive state, which the shell cannot copy across
// (it is a proxy, not plain data). The core takes JSON anyway, so this is
// exactly what arrives there.
const plain = (params: Record<string, unknown>): unknown => {
    return JSON.parse(JSON.stringify(params))
}

const call = async <T>(command: string, params: Record<string, unknown> = {}): Promise<T> => {
    return unwrap(await bridge().call<T>(command, plain(params)))
}

export const desktopApi: Api = {
    detectAccountSettings: async (email) => {
        try {
            const detection = await call<Detection>('detect_account_settings', { email })

            return { ok: true, detection }
        } catch (error) {
            if (isAddressError(error)) {
                return { ok: false, error }
            }

            throw error
        }
    },
    validateAccountSettings: (settings) => call<SettingsIssue[]>('validate_account_settings', { settings }),
    defaultPort: (protocol, security) => call<number>('default_port', { protocol, security }),

    listAccounts: () => call<Account[]>('list_accounts'),
    addAccount: (settings, password) => call<Account>('add_account', { settings, password }),
    beginSignIn: (settings) => call<SignIn>('begin_sign_in', { settings }),
    finishSignIn: (signInId) => call<Account>('finish_sign_in', { signInId }),
    cancelSignIn: (signInId) => call<void>('cancel_sign_in', { signInId }),
    removeAccount: (accountId) => call<void>('remove_account', { accountId }),

    listFolders: (accountId) => call<Folder[]>('list_folders', { accountId }),
    refreshFolders: (accountId) => call<Folder[]>('refresh_folders', { accountId }),

    listMessages: (folderId) => call<MessageSummary[]>('list_messages', { folderId }),
    syncFolder: (folderId, grow) => call<MessageSummary[]>('sync_folder', { folderId, grow }),
    openMessage: (messageId, allowRemote) => call<MessageContent>('open_message', { messageId, allowRemote }),
    setSeen: (messageIds, seen) => call<void>('set_seen', { messageIds, seen }),
    setFlagged: (messageIds, flagged) => call<void>('set_flagged', { messageIds, flagged }),
    deleteMessages: (messageIds) => call<void>('delete_messages', { messageIds }),
    moveMessages: (messageIds, folderId) => call<void>('move_messages', { messageIds, folderId }),

    replyDraft: (messageId, mode) => call<Draft>('reply_draft', { messageId, mode }),
    sendMessage: (message) => call<void>('send_message', { message }),

    saveAttachment: async (messageId, attachment) => {
        return unwrap(await bridge().saveAttachment(messageId, attachment.index, attachment.name))
    },
    openExternal: async (url) => {
        unwrap(await bridge().openExternal(url))
    },

    senderPicture: (email) => call<string | null>('sender_picture', { email }),

    settings: () => call<AppSettings>('settings'),
    setSettings: (settings) => call<void>('set_settings', { settings }),
}

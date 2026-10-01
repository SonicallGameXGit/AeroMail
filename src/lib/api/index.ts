// The only place in the UI that talks to the Rust core (AGENTS.md 5.2).
// Components receive an `Api` and never call Tauri directly, so the same UI
// can later run against another transport or a fake in tests.

import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import type { Account } from './generated/Account'
import type { AccountSettings } from './generated/AccountSettings'
import type { AddressError } from './generated/AddressError'
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

export type { Account } from './generated/Account'
export type { AccountSettings } from './generated/AccountSettings'
export type { AddressError } from './generated/AddressError'
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

export const tauriApi: Api = {
    detectAccountSettings: async (email) => {
        try {
            const detection = await invoke<Detection>('detect_account_settings', { email })

            return { ok: true, detection }
        } catch (error) {
            if (isAddressError(error)) {
                return { ok: false, error }
            }

            throw error
        }
    },
    validateAccountSettings: (settings) => invoke<SettingsIssue[]>('validate_account_settings', { settings }),
    defaultPort: (protocol, security) => invoke<number>('default_port', { protocol, security }),

    listAccounts: () => invoke<Account[]>('list_accounts'),
    addAccount: (settings, password) => invoke<Account>('add_account', { settings, password }),
    removeAccount: (accountId) => invoke<void>('remove_account', { accountId }),

    listFolders: (accountId) => invoke<Folder[]>('list_folders', { accountId }),
    refreshFolders: (accountId) => invoke<Folder[]>('refresh_folders', { accountId }),

    listMessages: (folderId) => invoke<MessageSummary[]>('list_messages', { folderId }),
    syncFolder: (folderId, grow) => invoke<MessageSummary[]>('sync_folder', { folderId, grow }),
    openMessage: (messageId, allowRemote) => invoke<MessageContent>('open_message', { messageId, allowRemote }),
    setSeen: (messageIds, seen) => invoke<void>('set_seen', { messageIds, seen }),
    setFlagged: (messageIds, flagged) => invoke<void>('set_flagged', { messageIds, flagged }),
    deleteMessages: (messageIds) => invoke<void>('delete_messages', { messageIds }),
    moveMessages: (messageIds, folderId) => invoke<void>('move_messages', { messageIds, folderId }),

    replyDraft: (messageId, mode) => invoke<Draft>('reply_draft', { messageId, mode }),
    sendMessage: (message) => invoke<void>('send_message', { message }),

    saveAttachment: async (messageId, attachment) => {
        const path = await save({ defaultPath: attachment.name })
        if (path === null) {
            return false
        }
        await invoke<void>('save_attachment', { messageId, index: attachment.index, path })

        return true
    },
    openExternal: (url) => openUrl(url),
}

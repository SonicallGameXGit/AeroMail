// In-memory `Api` for tests. It keeps a tiny mailbox and returns canned
// detection answers; the real rules are tested in the Rust core.

import type {
    Account,
    AccountSettings,
    Api,
    AppSettings,
    ColorScheme,
    Detection,
    Folder,
    FolderRole,
    MailError,
    MailEvent,
    NotificationRequest,
    NotificationTarget,
    MessageContent,
    MessageSummary,
    OutgoingMessage,
    SettingsIssue,
} from '$lib/api'
import { withAuth } from './setup'

export const gmailDetection: Detection = {
    source: 'BuiltIn',
    provider: 'Gmail',
    passwordKind: 'AppPassword',
    signInAvailable: false,
    settings: {
        email: 'john@gmail.com',
        displayName: '',
        incoming: {
            protocol: 'Imap',
            host: 'imap.gmail.com',
            port: 993,
            security: 'Tls',
            auth: 'Password',
            username: 'john@gmail.com',
        },
        outgoing: {
            protocol: 'Smtp',
            host: 'smtp.gmail.com',
            port: 465,
            security: 'Tls',
            auth: 'Password',
            username: 'john@gmail.com',
        },
    },
    pop3: {
        protocol: 'Pop3',
        host: 'pop.gmail.com',
        port: 995,
        security: 'Tls',
        auth: 'Password',
        username: 'john@gmail.com',
    },
}

// Gmail in a build that can sign in on Google's own page.
export const gmailSignInDetection: Detection = {
    ...gmailDetection,
    signInAvailable: true,
    settings: withAuth(gmailDetection.settings, 'OAuth2'),
}

export const guessDetection: Detection = {
    source: 'Guess',
    provider: null,
    passwordKind: 'Account',
    signInAvailable: false,
    settings: {
        email: 'me@corp.example',
        displayName: '',
        incoming: {
            protocol: 'Imap',
            host: 'imap.corp.example',
            port: 993,
            security: 'Tls',
            auth: 'Password',
            username: 'me@corp.example',
        },
        outgoing: {
            protocol: 'Smtp',
            host: 'smtp.corp.example',
            port: 465,
            security: 'Tls',
            auth: 'Password',
            username: 'me@corp.example',
        },
    },
    pop3: null,
}

// The account the user picks on the provider's page.
export const pickedAddress = 'picked@gmail.com'

export const fakePassword = 'secret'
export const fakeSignInUrl = 'https://accounts.example.com/sign-in'

export type FakeState = {
    accounts: Account[]
    folders: Folder[]
    messages: MessageSummary[]
    sent: OutgoingMessage[]
    opened: string[]
    // Sender pictures by address.
    pictures: Record<string, string>
    pictureRequests: string[]
    settings: AppSettings
    // Messages whose content brings remote images.
    remoteMessages: number[]
    // System notifications that were asked for.
    notified: NotificationRequest[]
    // What the shell was told to show its own windows and dialogs in, in order.
    colorSchemes: ColorScheme[]
    // Who listens for what the core announces / for clicks on notifications.
    listeners: ((event: MailEvent) => void)[]
    openListeners: ((target: NotificationTarget) => void)[]
}

// Plays what the core would announce by itself.
export const announce = (state: FakeState, event: MailEvent) => {
    for (const listener of state.listeners) {
        listener(event)
    }
}

// Plays a click on a notification.
export const clickNotification = (state: FakeState, target: NotificationTarget) => {
    for (const listener of state.openListeners) {
        listener(target)
    }
}

// What the core has for a fresh install.
export const defaultSettings = (): AppSettings => {
    return {
        senderPictures: true,
        remoteImages: false,
        switchToNewAccount: true,
        theme: 'System',
        notifications: {
            enabled: true,
            sound: true,
            whileFocused: false,
            duration: 'System',
            seconds: 10,
            title: '{sender}',
            body: '{subject}\n{preview}',
            previewLength: 90,
            icon: 'Sender',
            customIcon: null,
        },
    }
}

const fail = (kind: MailError['kind'], detail: string, server: MailError['server'] = null): MailError => {
    return { kind, server, detail }
}

const folder = (id: number, accountId: number, name: string, role: FolderRole): Folder => {
    return {
        id,
        accountId,
        path: name,
        name,
        depth: 0,
        role,
        selectable: true,
        unread: 0,
        total: 0,
    }
}

const message = (id: number, folderId: number, subject: string, sender: string, seen: boolean): MessageSummary => {
    return {
        id,
        folderId,
        uid: id,
        subject,
        from: [{ name: sender, email: sender.toLowerCase() + '@example.com' }],
        to: [{ name: null, email: 'john@gmail.com' }],
        cc: [],
        date: 1_700_000_000 + id,
        flags: { seen, flagged: false, answered: false },
        size: 1200,
    }
}

// One Gmail account with a few folders and two messages in the inbox.
export const seededState = (): FakeState => {
    return {
        accounts: [{ id: 1, settings: gmailDetection.settings }],
        folders: [
            folder(1, 1, 'INBOX', 'Inbox'),
            folder(2, 1, 'Sent', 'Sent'),
            folder(3, 1, 'Archive', 'Archive'),
            folder(4, 1, 'Trash', 'Trash'),
            folder(5, 1, 'Receipts', 'Other'),
        ],
        messages: [
            message(11, 1, 'Lunch on Friday?', 'Ann', false),
            message(10, 1, 'Quarterly report', 'Bob', true),
        ],
        sent: [],
        opened: [],
        pictures: {},
        pictureRequests: [],
        settings: defaultSettings(),
        remoteMessages: [],
        notified: [],
        colorSchemes: [],
        listeners: [],
        openListeners: [],
    }
}

export const emptyState = (): FakeState => {
    return {
        accounts: [],
        folders: [],
        messages: [],
        sent: [],
        opened: [],
        pictures: {},
        pictureRequests: [],
        settings: defaultSettings(),
        remoteMessages: [],
        notified: [],
        colorSchemes: [],
        listeners: [],
        openListeners: [],
    }
}

const counted = (state: FakeState, target: Folder): Folder => {
    const inside = state.messages.filter((candidate) => candidate.folderId === target.id)

    return {
        ...target,
        total: inside.length,
        unread: inside.filter((candidate) => !candidate.flags.seen).length,
    }
}

const find = (state: FakeState, messageId: number): MessageSummary => {
    const found = state.messages.find((candidate) => candidate.id === messageId)
    if (found === undefined) {
        throw fail('NotFound', 'message not found')
    }

    return found
}

const move = (state: FakeState, messageIds: number[], folderId: number) => {
    state.messages = state.messages.map((candidate) => {
        return messageIds.includes(candidate.id) ? { ...candidate, folderId } : candidate
    })
}

const htmlBodies: Record<number, string> = {
    10: '<p>Numbers attached. <a href="https://example.com/report">Open the report</a></p>',
}

export const createFakeApi = (state: FakeState = emptyState(), overrides: Partial<Api> = {}): Api => {
    const listFolders = async (accountId: number): Promise<Folder[]> => {
        return state.folders
            .filter((candidate) => candidate.accountId === accountId)
            .map((candidate) => counted(state, candidate))
    }

    const store = (settings: AccountSettings): Account => {
        const account = { id: state.accounts.length + 1, settings }
        state.accounts = [...state.accounts, account]
        state.folders = [...state.folders, folder(100 + account.id, account.id, 'INBOX', 'Inbox')]

        return account
    }
    // Sign-ins that were begun, by their number.
    const signIns: AccountSettings[] = []

    const listMessages = async (folderId: number): Promise<MessageSummary[]> => {
        return state.messages
            .filter((candidate) => candidate.folderId === folderId)
            .sort((a, b) => b.uid - a.uid)
    }

    return {
        detectAccountSettings: async (email) => {
            if (!email.includes('@')) {
                return { ok: false, error: 'MissingAt' }
            }
            if (email.endsWith('@gmail.com')) {
                return { ok: true, detection: gmailDetection }
            }

            return { ok: true, detection: guessDetection }
        },
        recognizeAddress: async (email) => {
            const domain = email.slice(email.lastIndexOf('@') + 1)
            if (!email.includes('@') || !domain.startsWith('gmail')) {
                return { type: 'unknown' }
            }
            if ('gmail.'.startsWith(domain) || domain === 'gmail.com') {
                return { type: 'known', brand: 'Gmail', provider: 'Gmail' }
            }
            if (domain.startsWith('gmail.')) {
                return { type: 'mistyped', provider: 'Gmail', suggestion: 'gmail.com' }
            }

            return { type: 'unknown' }
        },
        validateAccountSettings: async (settings) => {
            const issues: SettingsIssue[] = []
            if (settings.incoming.host === '') {
                issues.push({ server: 'Incoming', kind: 'EmptyHost', blocking: true })
            }

            return issues
        },
        defaultPort: async (protocol, security) => {
            if (protocol === 'Pop3') {
                return security === 'Tls' ? 995 : 110
            }
            if (protocol === 'Imap') {
                return security === 'Tls' ? 993 : 143
            }
            if (security === 'Tls') {
                return 465
            }

            return security === 'StartTls' ? 587 : 25
        },

        listAccounts: async () => state.accounts,
        addAccount: async (settings: AccountSettings, password: string) => {
            if (password !== fakePassword) {
                throw fail('Auth', 'invalid credentials', 'Incoming')
            }

            return store(settings)
        },
        beginSignIn: async (settings) => {
            signIns.push(settings)

            return { id: signIns.length, url: fakeSignInUrl }
        },
        beginChosenSignIn: async () => {
            const settings = withAuth(gmailDetection.settings, 'OAuth2')
            signIns.push({
                ...settings,
                email: pickedAddress,
                incoming: { ...settings.incoming, username: pickedAddress },
                outgoing: { ...settings.outgoing, username: pickedAddress },
            })

            return { id: signIns.length, url: fakeSignInUrl }
        },
        signInChoices: async () => [{ brand: 'Gmail', provider: 'Gmail' }],
        addressPresets: async () => [
            { provider: 'Gmail', domain: 'gmail.com', brand: 'Gmail' },
            { provider: 'Outlook', domain: 'outlook.com', brand: null },
        ],
        // The user approves in the browser at once.
        finishSignIn: async (signInId) => {
            const settings = signIns[signInId - 1]
            if (settings === undefined) {
                throw fail('SignIn', 'this sign-in was given up')
            }

            return store(settings)
        },
        cancelSignIn: async () => undefined,
        removeAccount: async (accountId) => {
            state.accounts = state.accounts.filter((candidate) => candidate.id !== accountId)
        },

        listFolders,
        refreshFolders: listFolders,

        listMessages,
        syncFolder: listMessages,
        openMessage: async (messageId, allowRemote): Promise<MessageContent> => {
            const summary = find(state, messageId)
            const remote = state.remoteMessages.includes(messageId)
            const blocked = remote && !allowRemote && !state.settings.remoteImages

            return {
                summary,
                text: `Body of ${summary.subject}`,
                html: remote ? '<p>Offers</p>' : htmlBodies[messageId] ?? null,
                attachments: [],
                remoteContentBlocked: blocked,
                messageId: `m${messageId}@example.com`,
                references: [],
                replyTo: [],
            }
        },
        setSeen: async (messageIds, seen) => {
            state.messages = state.messages.map((candidate) => {
                return messageIds.includes(candidate.id)
                    ? { ...candidate, flags: { ...candidate.flags, seen } }
                    : candidate
            })
        },
        setFlagged: async (messageIds, flagged) => {
            state.messages = state.messages.map((candidate) => {
                return messageIds.includes(candidate.id)
                    ? { ...candidate, flags: { ...candidate.flags, flagged } }
                    : candidate
            })
        },
        deleteMessages: async (messageIds) => {
            const trash = state.folders.find((candidate) => candidate.role === 'Trash')
            if (trash !== undefined) {
                move(state, messageIds, trash.id)
            }
        },
        moveMessages: async (messageIds, folderId) => {
            move(state, messageIds, folderId)
        },

        replyDraft: async (messageId, mode) => {
            const summary = find(state, messageId)
            const prefix = mode === 'Forward' ? 'Fwd: ' : 'Re: '

            return {
                to: mode === 'Forward' ? [] : summary.from.map((contact) => contact.email),
                cc: [],
                subject: prefix + summary.subject,
                body: `\n\n> Body of ${summary.subject}\n`,
                inReplyTo: mode === 'Forward' ? null : `m${messageId}@example.com`,
                references: [],
            }
        },
        sendMessage: async (outgoing) => {
            if (outgoing.to.length === 0) {
                throw fail('Invalid', 'the message has no recipients')
            }
            state.sent = [...state.sent, outgoing]
        },

        saveAttachment: async () => true,
        openExternal: async (url) => {
            state.opened = [...state.opened, url]
        },

        senderPicture: async (email) => {
            state.pictureRequests = [...state.pictureRequests, email]
            if (!state.settings.senderPictures) {
                return null
            }

            return state.pictures[email] ?? null
        },

        settings: async () => state.settings,
        setSettings: async (settings) => {
            state.settings = settings
        },
        defaultSettings: async () => defaultSettings(),

        onEvent: (listener) => {
            state.listeners = [...state.listeners, listener]

            return () => {
                state.listeners = state.listeners.filter((candidate) => candidate !== listener)
            }
        },
        notify: async (notification) => {
            state.notified = [...state.notified, notification]
        },
        setColorScheme: async (scheme) => {
            state.colorSchemes = [...state.colorSchemes, scheme]
        },
        onNotificationOpen: (listener) => {
            state.openListeners = [...state.openListeners, listener]

            return () => {
                state.openListeners = state.openListeners.filter((candidate) => candidate !== listener)
            }
        },
        ...overrides,
    }
}

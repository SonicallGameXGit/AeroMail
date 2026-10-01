// In-memory `Api` for tests. It keeps a tiny mailbox and returns canned
// detection answers; the real rules are tested in the Rust core.

import type {
    Account,
    AccountSettings,
    Api,
    Detection,
    Folder,
    FolderRole,
    MailError,
    MessageContent,
    MessageSummary,
    OutgoingMessage,
    SettingsIssue,
} from '$lib/api'

export const gmailDetection: Detection = {
    source: 'BuiltIn',
    provider: 'Gmail',
    passwordKind: 'AppPassword',
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

export const guessDetection: Detection = {
    source: 'Guess',
    provider: null,
    passwordKind: 'Account',
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

export const fakePassword = 'secret'

export type FakeState = {
    accounts: Account[]
    folders: Folder[]
    messages: MessageSummary[]
    sent: OutgoingMessage[]
    opened: string[]
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
    }
}

export const emptyState = (): FakeState => {
    return {
        accounts: [],
        folders: [],
        messages: [],
        sent: [],
        opened: [],
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
            const account = { id: state.accounts.length + 1, settings }
            state.accounts = [...state.accounts, account]
            state.folders = [...state.folders, folder(100 + account.id, account.id, 'INBOX', 'Inbox')]

            return account
        },
        removeAccount: async (accountId) => {
            state.accounts = state.accounts.filter((candidate) => candidate.id !== accountId)
        },

        listFolders,
        refreshFolders: listFolders,

        listMessages,
        syncFolder: listMessages,
        openMessage: async (messageId): Promise<MessageContent> => {
            const summary = find(state, messageId)

            return {
                summary,
                text: `Body of ${summary.subject}`,
                html: htmlBodies[messageId] ?? null,
                attachments: [],
                remoteContentBlocked: false,
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
        ...overrides,
    }
}

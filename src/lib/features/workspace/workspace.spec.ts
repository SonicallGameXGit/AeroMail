import { describe, expect, it } from 'vitest'
import type { Account, Conversation, Folder, MessageSummary } from '$lib/api'
import {
    answerTarget,
    contactFull,
    dayLabel,
    emptyForm,
    errorMessage,
    accountLabel,
    folderIcon,
    folderLabel,
    formFromDraft,
    frameDocument,
    groupFolders,
    inboxUnread,
    isOwnMessage,
    participantsLabel,
    pictureKey,
    isSafeLink,
    senderLabel,
    senderPicture,
    shortDate,
    sizeLabel,
    splitAddresses,
    subjectLabel,
    toOutgoing,
} from './workspace'

const folder = (overrides: Partial<Folder>): Folder => {
    return {
        id: 1,
        accountId: 1,
        path: 'INBOX',
        name: 'INBOX',
        depth: 0,
        role: 'Inbox',
        selectable: true,
        unread: 0,
        total: 0,
        ...overrides,
    }
}

const message = (overrides: Partial<MessageSummary>): MessageSummary => {
    return {
        id: 1,
        folderId: 1,
        uid: 1,
        subject: 'Hi',
        from: [{ name: 'Ann', email: 'ann@example.com' }],
        to: [],
        cc: [],
        date: 0,
        flags: { seen: false, flagged: false, answered: false },
        size: 0,
        ...overrides,
    }
}

describe('folders', () => {
    it('names well-known folders the same on every provider', () => {
        expect(folderLabel(folder({ name: 'INBOX', role: 'Inbox' }))).toBe('Inbox')
        expect(folderLabel(folder({ name: 'Sent Mail', role: 'Sent' }))).toBe('Sent')
        expect(folderLabel(folder({ name: 'Receipts', role: 'Other' }))).toBe('Receipts')
    })

    it('gives every well-known folder its own picture', () => {
        expect(folderIcon(folder({ role: 'Trash' }))).toBe('trash')
        expect(folderIcon(folder({ role: 'Inbox' }))).toBe('inbox')
        expect(folderIcon(folder({ role: 'Other' }))).toBe('folder')
    })

    it('shows well-known folders first, in one order for every account', () => {
        const groups = groupFolders([
            folder({ id: 1, role: 'All' }),
            folder({ id: 2, role: 'Trash' }),
            folder({ id: 3, role: 'Other', name: 'Notes' }),
            folder({ id: 4, role: 'Inbox' }),
            folder({ id: 5, role: 'Sent' }),
            folder({ id: 6, role: 'Junk', selectable: false }),
        ])

        expect(groups.main.map((entry) => entry.id)).toEqual([4, 5, 2, 1])
        expect(groups.custom.map((entry) => entry.folder.id)).toEqual([3])
    })

    it('indents a folder only under parents that are shown', () => {
        const groups = groupFolders([
            folder({ id: 1, role: 'Other', name: '[Gmail]', depth: 0, selectable: false }),
            folder({ id: 2, role: 'Other', name: 'Important', depth: 1 }),
            folder({ id: 3, role: 'Sent', name: 'Sent Mail', depth: 1 }),
            folder({ id: 4, role: 'Other', name: 'Work', depth: 0 }),
            folder({ id: 5, role: 'Other', name: 'Projects', depth: 1 }),
            folder({ id: 6, role: 'Other', name: 'Old', depth: 2 }),
            folder({ id: 7, role: 'Other', name: 'Notes', depth: 0 }),
        ])

        expect(groups.custom.map((entry) => [entry.folder.name, entry.indent])).toEqual([
            ['Important', 0],
            ['Work', 0],
            ['Projects', 1],
            ['Old', 2],
            ['Notes', 0],
        ])
    })

    it('counts unread mail of the inbox only', () => {
        expect(inboxUnread([
            folder({ role: 'Inbox', unread: 3 }),
            folder({ role: 'Junk', unread: 40 }),
        ])).toBe(3)
        expect(inboxUnread([])).toBe(0)
    })

    it('calls an account by its owner\'s name, or by its address', () => {
        const account = (displayName: string): Account => {
            const server = { host: 'x', port: 1, security: 'Tls', auth: 'Password', username: 'u' } as const

            return {
                id: 1,
                settings: {
                    email: 'john@gmail.com',
                    displayName,
                    incoming: { ...server, protocol: 'Imap' },
                    outgoing: { ...server, protocol: 'Smtp' },
                },
            }
        }

        expect(accountLabel(account('John'))).toBe('John')
        expect(accountLabel(account(' '))).toBe('john@gmail.com')
    })
})

describe('message labels', () => {
    it('falls back when sender or subject is missing', () => {
        expect(senderLabel(message({}))).toBe('Ann')
        expect(senderLabel(message({ from: [{ name: null, email: 'bob@example.com' }] }))).toBe('bob@example.com')
        expect(senderLabel(message({ from: [] }))).toBe('(unknown sender)')
        expect(subjectLabel(message({ subject: '' }))).toBe('(no subject)')
        expect(contactFull({ name: 'Ann', email: 'ann@example.com' })).toBe('Ann <ann@example.com>')
        expect(contactFull({ name: null, email: 'ann@example.com' })).toBe('ann@example.com')
    })

    it('finds a sender\'s picture whatever the case of the address', () => {
        const pictures = { [pictureKey(' Ann@Example.com ')]: 'data:image/svg+xml;base64,AAAA' }

        expect(senderPicture(pictures, message({}))).toBe('data:image/svg+xml;base64,AAAA')
        expect(senderPicture(pictures, message({ from: [{ name: 'Ann', email: 'ANN@example.com' }] }))).not.toBeNull()
        expect(senderPicture(pictures, message({ from: [{ name: 'Bob', email: 'bob@example.com' }] }))).toBeNull()
        expect(senderPicture(pictures, message({ from: [] }))).toBeNull()
    })

    it('shortens dates by how old they are', () => {
        const now = new Date(2026, 9, 1, 15, 0)
        const at = (date: Date) => date.getTime() / 1000

        expect(shortDate(at(new Date(2026, 9, 1, 9, 5)), now, 'en-GB')).toBe('09:05')
        expect(shortDate(at(new Date(2026, 2, 7, 9, 5)), now, 'en-GB')).toBe('7 Mar')
        expect(shortDate(at(new Date(2024, 2, 7, 9, 5)), now, 'en-GB')).toBe('7 Mar 2024')
    })

    it('formats sizes', () => {
        expect(sizeLabel(512)).toBe('512 B')
        expect(sizeLabel(2048)).toBe('2 KB')
        expect(sizeLabel(5 * 1024 * 1024)).toBe('5.0 MB')
    })
})

describe('compose form', () => {
    it('splits typed recipients', () => {
        expect(splitAddresses(' a@x.com, b@x.com;c@x.com\n ,')).toEqual(['a@x.com', 'b@x.com', 'c@x.com'])
        expect(splitAddresses('')).toEqual([])
    })

    it('turns the form into an outgoing message', () => {
        const form = { ...emptyForm(3), to: 'a@x.com, b@x.com', bcc: 'c@x.com', subject: 'S', body: 'B' }

        expect(toOutgoing(form)).toEqual({
            accountId: 3,
            to: ['a@x.com', 'b@x.com'],
            cc: [],
            bcc: ['c@x.com'],
            subject: 'S',
            body: 'B',
            inReplyTo: null,
            references: [],
        })
    })

    it('starts from a reply draft', () => {
        const form = formFromDraft(2, {
            to: ['a@x.com', 'b@x.com'],
            cc: ['c@x.com'],
            subject: 'Re: S',
            body: '> quoted',
            inReplyTo: 'm1@x.com',
            references: ['m0@x.com', 'm1@x.com'],
        })

        expect(form.to).toBe('a@x.com, b@x.com')
        expect(form.cc).toBe('c@x.com')
        expect(toOutgoing(form).inReplyTo).toBe('m1@x.com')
        expect(toOutgoing(form).references).toEqual(['m0@x.com', 'm1@x.com'])
    })
})

describe('errors', () => {
    it('names the side that failed', () => {
        expect(errorMessage({ kind: 'Auth', server: 'Incoming', detail: '' })).toBe(
            'Incoming mail: The server didn\'t accept the username or password.',
        )
        expect(errorMessage({ kind: 'Keychain', server: null, detail: '' })).toContain('keychain')
    })
})

describe('message frame', () => {
    it('never allows scripts and blocks remote images and fonts unless allowed', () => {
        const blocked = frameDocument('<p>Hi</p>', false)
        expect(blocked).toContain('default-src \'none\'')
        expect(blocked).toContain('style-src \'unsafe-inline\';')
        expect(blocked).toContain('font-src data:;')
        expect(blocked).toContain('img-src data:"')
        expect(blocked).not.toContain('script-src')
        expect(blocked).toContain('<body><p>Hi</p></body>')

        const allowed = frameDocument('<p>Hi</p>', true)
        expect(allowed).toContain('style-src \'unsafe-inline\' http: https:;')
        expect(allowed).toContain('font-src data: http: https:;')
        expect(allowed).toContain('img-src data: http: https:"')
        expect(allowed).not.toContain('script-src')
    })

    it('leaves the layout of a message to the message', () => {
        const page = frameDocument('<p>Hi</p>', false)

        expect(page).not.toContain('max-width')
        expect(page).not.toContain('height:auto')
        expect(page).not.toContain('overflow-wrap:anywhere')
    })

    it('hides what a reply quotes only when asked to', () => {
        expect(frameDocument('<p>Hi</p>', false)).not.toContain('.gmail_quote')
        expect(frameDocument('<p>Hi</p>', false, true)).toContain('.gmail_quote, .gmail_quote_container, blockquote')
        expect(frameDocument('<p>Hi</p>', false, true)).toContain('{display:none!important}')
    })

    it('lets only web and mail links leave the app', () => {
        expect(isSafeLink('https://example.com')).toBe(true)
        expect(isSafeLink('HTTP://example.com')).toBe(true)
        expect(isSafeLink('mailto:ann@example.com')).toBe(true)
        expect(isSafeLink('javascript:alert(1)')).toBe(false)
        expect(isSafeLink('file:///etc/passwd')).toBe(false)
        expect(isSafeLink('/relative')).toBe(false)
    })
})

describe('conversations', () => {
    const me = 'John@Gmail.com'
    const ann = { name: 'Ann Lee', email: 'ann@example.com' }
    const john = { name: 'John', email: 'john@gmail.com' }

    const conversation = (overrides: Partial<Conversation>): Conversation => {
        return {
            latest: message({ from: [ann] }),
            messageIds: [1],
            count: 1,
            unread: 0,
            flagged: false,
            senders: [ann],
            ...overrides,
        }
    }

    it('knows the account\'s own messages whatever the case of the address', () => {
        expect(isOwnMessage(message({ from: [john] }), me)).toBe(true)
        expect(isOwnMessage(message({ from: [ann] }), me)).toBe(false)
        expect(isOwnMessage(message({ from: [] }), me)).toBe(false)
    })

    it('names one sender in full and several by first name, the account as me', () => {
        expect(participantsLabel(conversation({}), me)).toBe('Ann Lee')
        expect(participantsLabel(conversation({ senders: [ann, john, { name: null, email: 'bob@example.com' }] }), me))
            .toBe('Ann, me, bob')
    })

    it('answers the newest message someone else wrote', () => {
        const thread = [
            message({ id: 1, from: [ann] }),
            message({ id: 2, from: [{ name: 'Bob', email: 'bob@example.com' }] }),
            message({ id: 3, from: [john] }),
        ]

        expect(answerTarget(thread, me)?.id).toBe(2)
        expect(answerTarget([message({ id: 7, from: [john] })], me)?.id).toBe(7)
        expect(answerTarget([], me)).toBeNull()
    })

    it('names the days of a conversation the way people say them', () => {
        const now = new Date(2026, 9, 11, 15, 0)
        const at = (day: number, month = 9, year = 2026) => new Date(year, month, day, 9, 30).getTime() / 1000

        expect(dayLabel(at(11), now)).toBe('Today')
        expect(dayLabel(at(10), now)).toBe('Yesterday')
        expect(dayLabel(at(5), now, 'en-GB')).toBe('Monday 5 October')
        expect(dayLabel(at(5, 9, 2025), now, 'en-GB')).toBe('5 October 2025')
    })
})

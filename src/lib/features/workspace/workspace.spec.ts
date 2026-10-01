import { describe, expect, it } from 'vitest'
import type { Folder, MessageSummary } from '$lib/api'
import {
    contactFull,
    emptyForm,
    errorMessage,
    folderIndent,
    folderLabel,
    formFromDraft,
    frameDocument,
    isSafeLink,
    senderLabel,
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

    it('indents only ordinary folders', () => {
        expect(folderIndent(folder({ role: 'Sent', depth: 1 }))).toBe(0)
        expect(folderIndent(folder({ role: 'Other', depth: 2 }))).toBe(2)
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
    it('never allows scripts and blocks remote images unless allowed', () => {
        const blocked = frameDocument('<p>Hi</p>', false)
        expect(blocked).toContain('default-src \'none\'')
        expect(blocked).toContain('img-src data:"')
        expect(blocked).not.toContain('script-src')
        expect(blocked).toContain('<body><p>Hi</p></body>')

        const allowed = frameDocument('<p>Hi</p>', true)
        expect(allowed).toContain('img-src data: http: https:"')
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

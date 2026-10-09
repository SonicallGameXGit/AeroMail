import { describe, expect, it } from 'vitest'
import type { NewMailItem, NotificationSettings } from '$lib/api'
import { defaultSettings } from '$lib/features/accounts/fakeApi'
import {
    bodyPresets,
    fillTemplate,
    newMailText,
    notificationIcon,
    presetOf,
    shorten,
    titlePresets,
    type NewMail,
} from './notification'

const arrived = (messages: NewMailItem[]): NewMail => {
    return { type: 'newMail', accountId: 1, folderId: 2, account: 'john@gmail.com', messages }
}

const item = (id: number, subject: string, name: string | null, preview: string | null = null): NewMailItem => {
    return { id, subject, from: { name, email: 'ann@example.com' }, preview }
}

const settings = (change: Partial<NotificationSettings> = {}): NotificationSettings => {
    return { ...defaultSettings().notifications, ...change }
}

const values = {
    sender: 'Ann',
    address: 'ann@example.com',
    subject: 'Hi',
    preview: '',
    account: 'me@example.com',
}

describe('fillTemplate', () => {
    it('fills the fields it knows and leaves the rest as written', () => {
        expect(fillTemplate('{sender} <{address}> {unknown}', values)).toBe('Ann <ann@example.com> {unknown}')
    })

    it('leaves out a line that turned out empty', () => {
        expect(fillTemplate('{subject}\n{preview}', values)).toBe('Hi')
        expect(fillTemplate('', values)).toBe('')
    })
})

describe('shorten', () => {
    it('keeps a short text as it is', () => {
        expect(shorten('Hello there', 20)).toBe('Hello there')
    })

    it('ends a cut text with an ellipsis, between words when it can', () => {
        expect(shorten('Are we still on for lunch on Friday?', 20)).toBe('Are we still on for…')
        expect(shorten('Supercalifragilisticexpialidocious', 10)).toBe('Supercalif…')
    })
})

describe('newMailText', () => {
    it('says by default who wrote, what about and how it starts', () => {
        const event = arrived([item(7, 'Lunch on Friday?', 'Ann', 'Hi! Are we still on for Friday? I found a new place near the office.')])

        expect(newMailText(event, settings({ previewLength: 30 }))).toEqual({
            title: 'Ann',
            body: 'Lunch on Friday?\nHi! Are we still on for…',
        })
    })

    it('follows the user\'s own templates', () => {
        const event = arrived([item(7, 'Hi', null)])

        expect(newMailText(event, settings({ title: '{address}', body: 'To {account}' }))).toEqual({
            title: 'ann@example.com',
            body: 'To john@gmail.com',
        })
    })

    it('has words for a missing sender and subject, and a title that is never empty', () => {
        const event = arrived([{ id: 7, subject: '', from: null, preview: null }])

        expect(newMailText(event, settings())).toEqual({ title: '(unknown sender)', body: '(no subject)' })
        expect(newMailText(event, settings({ title: '{preview}' }))?.title).toBe('MailClient')
    })

    it('lists a few of many and counts the rest', () => {
        const event = arrived([1, 2, 3, 4, 5].map((id) => item(id, 'Subject ' + id, 'Ann')))

        expect(newMailText(event, settings())).toEqual({
            title: '5 new messages',
            body: 'Ann: Subject 1\nAnn: Subject 2\nAnn: Subject 3\n…and 2 more',
        })
    })

    it('has nothing to say about no messages', () => {
        expect(newMailText(arrived([]), settings())).toBeNull()
    })
})

describe('notificationIcon', () => {
    const nobody = async () => null

    it('gives the app icon, no picture or the user\'s own one as chosen', async () => {
        const event = arrived([item(7, 'Hi', 'Ann')])
        const own = 'data:image/png;base64,AAAA'

        expect(await notificationIcon(event, settings({ icon: 'App' }), nobody)).toEqual({ kind: 'app' })
        expect(await notificationIcon(event, settings({ icon: 'None' }), nobody)).toEqual({ kind: 'none' })
        expect(await notificationIcon(event, settings({ icon: 'Custom', customIcon: own }), nobody)).toEqual({ kind: 'image', url: own })
        // Chosen, but no picture was picked yet.
        expect(await notificationIcon(event, settings({ icon: 'Custom' }), nobody)).toEqual({ kind: 'app' })
    })

    it('shows no one face for mail from several senders', async () => {
        const event = arrived([
            item(1, 'Hi', 'Ann'),
            { id: 2, subject: 'Yo', from: { name: 'Bob', email: 'bob@example.com' }, preview: null },
        ])

        expect(await notificationIcon(event, settings({ icon: 'Sender' }), nobody)).toEqual({ kind: 'app' })
    })
})

describe('presetOf', () => {
    it('recognises a ready-made template and calls anything else the user\'s own', () => {
        expect(presetOf(titlePresets, '{subject}')).toBe('subject')
        expect(presetOf(bodyPresets, '{subject}\n{preview}')).toBe('subjectPreview')
        expect(presetOf(bodyPresets, 'From {sender}')).toBe('custom')
    })
})

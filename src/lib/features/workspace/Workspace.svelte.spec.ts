import { describe, expect, it } from 'vitest'
import { page } from 'vitest/browser'
import { render } from 'vitest-browser-svelte'
import { createFakeApi, emptyState, fakePassword, seededState } from '$lib/features/accounts/fakeApi'
import Workspace from './Workspace.svelte'

const start = (overrides: Parameters<typeof createFakeApi>[1] = {}) => {
    const state = seededState()
    render(Workspace, { api: createFakeApi(state, overrides) })

    return state
}

const row = (subject: string) => page.getByRole('button', { name: subject })
const folder = (name: string) => page.getByRole('navigation').getByRole('button', { name })

describe('Workspace', () => {
    it('asks for an account when there is none, then opens the workspace', async () => {
        const state = emptyState()
        render(Workspace, { api: createFakeApi(state) })

        await page.getByLabelText('Email address').fill('john@gmail.com')
        await page.getByRole('button', { name: 'Continue' }).click()
        await page.getByLabelText('Password', { exact: true }).fill(fakePassword)
        await page.getByRole('button', { name: 'Connect' }).click()

        await expect.element(folder('Inbox')).toBeInTheDocument()
        await expect.element(page.getByText('john@gmail.com')).toBeInTheDocument()
    })

    it('opens the inbox with its messages and the unread count', async () => {
        start()

        await expect.element(row('Lunch on Friday?')).toBeInTheDocument()
        await expect.element(row('Quarterly report')).toBeInTheDocument()
        await expect.element(folder('Inbox')).toHaveTextContent('1')
        await expect.element(page.getByText('Select a message to read it.')).toBeInTheDocument()
    })

    it('shows a message and marks it read when it is opened', async () => {
        const state = start()

        await row('Lunch on Friday?').click()

        await expect.element(page.getByRole('heading', { name: 'Lunch on Friday?' })).toBeInTheDocument()
        await expect.element(page.getByText('Body of Lunch on Friday?')).toBeInTheDocument()
        await expect.element(page.getByText('Ann <ann@example.com>')).toBeInTheDocument()
        await expect.element(page.getByRole('button', { name: 'Mark unread' })).toBeEnabled()
        expect(state.messages.find((message) => message.id === 11)?.flags.seen).toBe(true)
    })

    it('marks a message unread again and flags it', async () => {
        const state = start()

        await row('Quarterly report').click()
        await page.getByRole('button', { name: 'Mark unread' }).click()
        await expect.element(page.getByRole('button', { name: 'Mark read' })).toBeInTheDocument()

        await page.getByRole('button', { name: 'Flag', exact: true }).click()
        await expect.element(page.getByRole('button', { name: 'Unflag' })).toBeInTheDocument()

        const message = state.messages.find((candidate) => candidate.id === 10)
        expect(message?.flags).toEqual({ seen: false, flagged: true, answered: false })
    })

    it('deletes a message and moves on to the next one', async () => {
        const state = start()

        await row('Lunch on Friday?').click()
        await page.getByRole('button', { name: 'Delete' }).click()

        await expect.element(row('Lunch on Friday?')).not.toBeInTheDocument()
        await expect.element(page.getByRole('heading', { name: 'Quarterly report' })).toBeInTheDocument()
        expect(state.messages.find((message) => message.id === 11)?.folderId).toBe(4)

        await folder('Trash').click()
        await expect.element(row('Lunch on Friday?')).toBeInTheDocument()
    })

    it('archives and moves messages to other folders', async () => {
        const state = start()

        await row('Lunch on Friday?').click()
        await page.getByRole('toolbar').getByRole('button', { name: 'Archive' }).click()
        await expect.element(row('Lunch on Friday?')).not.toBeInTheDocument()
        expect(state.messages.find((message) => message.id === 11)?.folderId).toBe(3)

        await page.getByRole('combobox', { name: 'Move to…' }).selectOptions('Receipts')
        await expect.element(row('Quarterly report')).not.toBeInTheDocument()
        expect(state.messages.find((message) => message.id === 10)?.folderId).toBe(5)
        await expect.element(page.getByText('No messages here.')).toBeInTheDocument()
    })

    it('writes and sends a new message', async () => {
        const state = start()

        await page.getByRole('button', { name: 'New message' }).click()
        await page.getByLabelText('To', { exact: true }).fill('ann@example.com, bob@example.com')
        await page.getByLabelText('Subject').fill('Hello')
        await page.getByLabelText('Message', { exact: true }).fill('How are you?')
        await page.getByRole('button', { name: 'Send' }).click()

        await expect.element(page.getByRole('dialog')).not.toBeInTheDocument()
        expect(state.sent).toEqual([{
            accountId: 1,
            to: ['ann@example.com', 'bob@example.com'],
            cc: [],
            bcc: [],
            subject: 'Hello',
            body: 'How are you?',
            inReplyTo: null,
            references: [],
        }])
    })

    it('keeps the composer open and explains when sending fails', async () => {
        const state = start()

        await page.getByRole('button', { name: 'New message' }).click()
        await page.getByLabelText('Subject').fill('Nobody')
        await page.getByRole('button', { name: 'Send' }).click()

        await expect.element(page.getByRole('dialog').getByRole('alert')).toHaveTextContent('the message has no recipients')
        await expect.element(page.getByLabelText('Subject')).toHaveValue('Nobody')
        expect(state.sent).toEqual([])
    })

    it('starts a reply from the open message', async () => {
        const state = start()

        await row('Lunch on Friday?').click()
        await page.getByRole('button', { name: 'Reply', exact: true }).click()

        await expect.element(page.getByLabelText('To', { exact: true })).toHaveValue('ann@example.com')
        await expect.element(page.getByLabelText('Subject')).toHaveValue('Re: Lunch on Friday?')

        await page.getByRole('button', { name: 'Send' }).click()
        await expect.element(page.getByRole('dialog')).not.toBeInTheDocument()
        expect(state.sent[0]?.inReplyTo).toBe('m11@example.com')
    })

    it('discards a message without sending it', async () => {
        const state = start()

        await page.getByRole('button', { name: 'New message' }).click()
        await page.getByLabelText('Subject').fill('Never mind')
        await page.getByRole('button', { name: 'Discard' }).click()

        await expect.element(page.getByRole('dialog')).not.toBeInTheDocument()
        expect(state.sent).toEqual([])
    })

    it('shows server errors in a bar that can be dismissed', async () => {
        start({
            syncFolder: async () => {
                throw { kind: 'Connect', server: 'Incoming', detail: 'imap.gmail.com:993: no answer' }
            },
        })

        await expect.element(page.getByRole('alert')).toHaveTextContent('Incoming mail: The server couldn\'t be reached')
        // Cached mail stays readable while offline.
        await expect.element(row('Lunch on Friday?')).toBeInTheDocument()

        await page.getByRole('button', { name: 'Dismiss' }).click()
        await expect.element(page.getByRole('alert')).not.toBeInTheDocument()
    })

    it('removes an account only after confirmation', async () => {
        const state = start()

        await page.getByRole('button', { name: 'Remove account: john@gmail.com' }).click()
        await page.getByRole('button', { name: 'Keep' }).click()
        expect(state.accounts.length).toBe(1)

        await page.getByRole('button', { name: 'Remove account: john@gmail.com' }).click()
        await page.getByRole('button', { name: 'Remove', exact: true }).click()

        await expect.element(page.getByRole('heading', { name: 'Add your email account' })).toBeInTheDocument()
        expect(state.accounts).toEqual([])
    })

    it('opens links from a message in the system browser, not in the app', async () => {
        const state = start()

        await row('Quarterly report').click()
        const frame = page.frameLocator(page.getByTitle('Message content'))
        await frame.getByRole('link', { name: 'Open the report' }).click()

        await expect.poll(() => state.opened).toEqual(['https://example.com/report'])
    })
})

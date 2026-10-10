import { describe, expect, it } from 'vitest'
import { page, userEvent } from 'vitest/browser'
import { render } from 'vitest-browser-svelte'
import { announce, clickNotification, createFakeApi, defaultSettings, emptyState, fakePassword, guessDetection, seededState } from '$lib/features/accounts/fakeApi'
import '$lib/design/base.css'
import Workspace from './Workspace.svelte'

const start = (overrides: Parameters<typeof createFakeApi>[1] = {}) => {
    const state = seededState()
    render(Workspace, { api: createFakeApi(state, overrides) })

    return state
}

const row = (subject: string) => page.getByRole('button', { name: subject })
const folder = (name: string) => page.getByRole('navigation').getByRole('button', { name })
// A one-pixel picture that really loads.
const pixel = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw=='
const pictureOf = (subject: string) => row(subject).element().querySelector('img')
const toolbar = () => page.getByRole('toolbar', { name: 'Message actions' })
// The window remembers the category opened last, so each test names its own.
const openSettings = async (category: 'Accounts' | 'Appearance' | 'Privacy' | 'Notifications' = 'Accounts') => {
    await page.getByRole('button', { name: /^Account: / }).click()
    await page.getByRole('menuitem', { name: 'Settings' }).click()
    await page.getByRole('tab', { name: category }).click()
}
const settings = () => page.getByRole('dialog', { name: 'Settings' })
const addedBox = () => page.getByRole('dialog', { name: 'Account added' })
const addCorpAccount = async () => {
    await page.getByRole('button', { name: 'Add account' }).click()
    await page.getByLabelText('Email address').fill('me@corp.example')
    await page.getByRole('button', { name: 'Continue' }).click()
    await page.getByLabelText('Password', { exact: true }).fill(fakePassword)
    await page.getByRole('button', { name: 'Connect' }).click()
}

describe('Workspace', () => {
    it('asks for an account when there is none, then opens the workspace', async () => {
        const state = emptyState()
        render(Workspace, { api: createFakeApi(state) })

        await page.getByLabelText('Email address').fill('john@gmail.com')
        await page.getByRole('button', { name: 'Continue' }).click()
        await page.getByLabelText('Password', { exact: true }).fill(fakePassword)
        await page.getByRole('button', { name: 'Connect' }).click()

        await expect.element(addedBox().getByRole('heading', { name: 'john@gmail.com is ready' })).toBeInTheDocument()
        await expect.element(addedBox()).toHaveTextContent('The account is added and open now.')
        await expect.element(folder('Inbox')).toBeInTheDocument()
        await expect.element(page.getByRole('button', { name: 'Account: john@gmail.com' })).toBeInTheDocument()

        await addedBox().getByRole('button', { name: 'OK' }).click()
        await expect.element(addedBox()).not.toBeInTheDocument()
    })

    it('closes the settings when an account added there is opened, then says it was added', async () => {
        start()

        await openSettings()
        await addCorpAccount()

        await expect.element(addedBox().getByRole('heading', { name: 'me@corp.example is ready' })).toBeInTheDocument()
        await expect.element(addedBox()).toHaveTextContent('The account is added and open now.')
        await expect.element(settings()).not.toBeInTheDocument()
        await expect.element(page.getByRole('button', { name: 'Account: me@corp.example' })).toBeInTheDocument()

        // Like a system message box: OK has the focus, Enter takes it.
        await expect.element(addedBox().getByRole('button', { name: 'OK' })).toHaveFocus()
        await userEvent.keyboard('{Enter}')
        await expect.element(addedBox()).not.toBeInTheDocument()
    })

    it('keeps the settings and the open account when new accounts are not opened', async () => {
        const state = seededState()
        state.settings = { ...state.settings, switchToNewAccount: false }
        render(Workspace, { api: createFakeApi(state) })

        await openSettings()
        await addCorpAccount()

        await expect.element(addedBox().getByRole('heading', { name: 'me@corp.example is ready' })).toBeInTheDocument()
        await expect.element(addedBox()).toHaveTextContent('The account is added. You can open it from the account menu at the top right.')
        await userEvent.keyboard('{Escape}')
        await expect.element(addedBox()).not.toBeInTheDocument()

        await expect.element(settings().getByRole('button', { name: 'Remove account: me@corp.example' })).toBeInTheDocument()
        await expect.element(page.getByRole('button', { name: 'Account: john@gmail.com', exact: true })).toBeInTheDocument()
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

        await toolbar().getByRole('button', { name: 'Flag', exact: true }).click()
        await expect.element(toolbar().getByRole('button', { name: 'Unflag' })).toBeInTheDocument()

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
        await toolbar().getByRole('button', { name: 'Archive' }).click()
        await expect.element(row('Lunch on Friday?')).not.toBeInTheDocument()
        expect(state.messages.find((message) => message.id === 11)?.folderId).toBe(3)

        await page.getByRole('button', { name: 'Move to…' }).click()
        await page.getByRole('menuitem', { name: 'Receipts' }).click()
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

        await openSettings()
        await settings().getByRole('button', { name: 'Remove account: john@gmail.com' }).click()
        await page.getByRole('button', { name: 'Keep' }).click()
        await expect.element(page.getByRole('dialog', { name: 'Remove account' })).not.toBeInTheDocument()
        expect(state.accounts.length).toBe(1)

        await settings().getByRole('button', { name: 'Remove account: john@gmail.com' }).click()
        await page.getByRole('dialog', { name: 'Remove account' }).getByRole('button', { name: 'Remove', exact: true }).click()

        await expect.element(page.getByRole('heading', { name: 'Add your email account' })).toBeInTheDocument()
        expect(state.accounts).toEqual([])
    })

    it('removes one of several accounts from the settings and keeps the others', async () => {
        const state = seededState()
        state.accounts.push({ id: 2, settings: { ...guessDetection.settings } })
        state.folders.push({ id: 20, accountId: 2, path: 'INBOX', name: 'INBOX', depth: 0, role: 'Inbox', selectable: true, unread: 0, total: 0 })
        render(Workspace, { api: createFakeApi(state) })

        await openSettings()
        await settings().getByRole('button', { name: 'Remove account: john@gmail.com' }).click()
        await page.getByRole('dialog', { name: 'Remove account' }).getByRole('button', { name: 'Remove', exact: true }).click()

        await expect.element(settings().getByRole('button', { name: 'Remove account: john@gmail.com' })).not.toBeInTheDocument()
        await expect.element(settings().getByRole('button', { name: 'Remove account: me@corp.example' })).toBeInTheDocument()
        expect(state.accounts.map((account) => account.id)).toEqual([2])

        await page.getByRole('button', { name: 'Done' }).click()
        await expect.element(page.getByRole('button', { name: 'Account: me@corp.example' })).toBeInTheDocument()
    })

    it('flags a message from its row in the list', async () => {
        const state = start()

        await page.getByRole('list', { name: 'Messages' }).getByRole('button', { name: 'Flag', exact: true }).first().click()

        await expect.element(page.getByRole('button', { name: 'Unflag' })).toBeInTheDocument()
        expect(state.messages.find((message) => message.id === 11)?.flags.flagged).toBe(true)
        // Flagging is not reading.
        expect(state.messages.find((message) => message.id === 11)?.flags.seen).toBe(false)
    })

    it('offers the message actions only when a message is open', async () => {
        start()

        await expect.element(row('Lunch on Friday?')).toBeInTheDocument()
        await expect.element(toolbar()).not.toBeInTheDocument()

        await row('Lunch on Friday?').click()
        await expect.element(toolbar().getByRole('button', { name: 'Reply all' })).toBeEnabled()
        await expect.element(toolbar().getByRole('button', { name: 'Forward' })).toBeEnabled()
    })

    it('switches between accounts from the account menu', async () => {
        const state = seededState()
        state.accounts.push({ id: 2, settings: { ...guessDetection.settings } })
        const second = { accountId: 2, depth: 0, selectable: true, unread: 0, total: 0 }
        state.folders.push({ ...second, id: 20, path: 'INBOX', name: 'INBOX', role: 'Inbox' })
        state.folders.push({ ...second, id: 21, path: 'Clients', name: 'Clients', role: 'Other' })
        render(Workspace, { api: createFakeApi(state) })

        await expect.element(row('Lunch on Friday?')).toBeInTheDocument()

        await page.getByRole('button', { name: 'Account: john@gmail.com' }).click()
        await page.getByRole('menuitem', { name: 'me@corp.example' }).click()

        await expect.element(page.getByRole('button', { name: 'Account: me@corp.example' })).toBeInTheDocument()
        await expect.element(folder('Clients')).toBeInTheDocument()
        await expect.element(folder('Receipts')).not.toBeInTheDocument()
        await expect.element(page.getByText('No messages here.')).toBeInTheDocument()
    })

    it('resizes the panes with the keyboard and remembers the widths', async () => {
        localStorage.clear()
        start()

        const edge = page.getByRole('separator', { name: 'Resize the folder list' })
        await expect.element(edge).toHaveAttribute('aria-valuenow', '220')

        edge.element().focus()
        await userEvent.keyboard('{ArrowRight}{ArrowRight}')
        await expect.element(edge).toHaveAttribute('aria-valuenow', '252')
        expect(page.getByRole('navigation').element().getBoundingClientRect().width).toBe(252)
        expect(localStorage.getItem('workspace.layout')).toContain('252')

        // All the way down is the icon rail: folders stay reachable by name.
        await userEvent.keyboard('{Home}')
        await expect.element(edge).toHaveAttribute('aria-valuenow', '52')
        await expect.element(folder('Trash')).toBeInTheDocument()
        localStorage.clear()
    })

    it('shows a sender\'s own picture, and the initial for a sender without one', async () => {
        const state = seededState()
        state.pictures = { 'bob@example.com': pixel }
        render(Workspace, { api: createFakeApi(state) })

        await expect.poll(() => pictureOf('Quarterly report')?.getAttribute('src')).toBe(pixel)
        expect(pictureOf('Lunch on Friday?')).toBeNull()
        await expect.element(row('Lunch on Friday?')).toHaveTextContent('A')

        // The same sender is not asked about again when the list is refreshed.
        await page.getByRole('button', { name: 'Refresh' }).click()
        await row('Quarterly report').click()
        await expect.element(page.getByRole('heading', { name: 'Quarterly report' })).toBeInTheDocument()
        expect(state.pictureRequests.toSorted()).toEqual(['ann@example.com', 'bob@example.com'])
    })

    it('stops showing and asking for sender pictures when they are turned off', async () => {
        const state = seededState()
        state.pictures = { 'bob@example.com': pixel }
        render(Workspace, { api: createFakeApi(state) })
        await expect.poll(() => pictureOf('Quarterly report')).not.toBeNull()

        await openSettings('Privacy')
        await page.getByRole('checkbox', { name: 'Show sender pictures' }).click()
        await page.getByRole('button', { name: 'Done' }).click()

        await expect.poll(() => pictureOf('Quarterly report')).toBeNull()
        expect(state.settings.senderPictures).toBe(false)
        const before = state.pictureRequests.length
        await page.getByRole('button', { name: 'Refresh' }).click()
        await expect.element(row('Quarterly report')).toHaveTextContent('B')
        expect(state.pictureRequests.length).toBe(before)

        await openSettings('Privacy')
        await expect.element(page.getByRole('checkbox', { name: 'Show sender pictures' })).not.toBeChecked()
        await page.getByRole('checkbox', { name: 'Show sender pictures' }).click()
        await expect.poll(() => pictureOf('Quarterly report')).not.toBeNull()
    })

    it('asks before showing remote images, unless the settings say always', async () => {
        const state = seededState()
        state.remoteMessages = [10, 11]
        render(Workspace, { api: createFakeApi(state) })
        const hidden = page.getByText('Remote images and fonts are hidden to protect your privacy.')
        const policy = () => page.getByTitle('Message content').element().getAttribute('srcdoc') ?? ''

        await row('Quarterly report').click()
        await expect.element(hidden).toBeInTheDocument()
        expect(policy()).toContain('img-src data:"')

        await page.getByRole('button', { name: 'Show images' }).click()
        await expect.element(hidden).not.toBeInTheDocument()

        // Asking for one message says nothing about the next one.
        await row('Lunch on Friday?').click()
        await expect.element(hidden).toBeInTheDocument()

        await openSettings('Privacy')
        await page.getByRole('checkbox', { name: 'Always show remote images' }).click()
        await page.getByRole('button', { name: 'Done' }).click()
        await expect.element(hidden).not.toBeInTheDocument()
        expect(state.settings).toEqual({ ...defaultSettings(), remoteImages: true })
        expect(policy()).toContain('img-src data: http: https:"')

        await row('Quarterly report').click()
        await expect.element(page.getByRole('heading', { name: 'Quarterly report' })).toBeInTheDocument()
        await expect.element(hidden).not.toBeInTheDocument()

        await openSettings('Privacy')
        await page.getByRole('checkbox', { name: 'Always show remote images' }).click()
        await expect.element(hidden).toBeInTheDocument()
    })

    it('takes a setting back and says so when it cannot be stored', async () => {
        const state = start({
            setSettings: async () => {
                throw { kind: 'Storage', server: null, detail: 'disk full' }
            },
        })

        await openSettings('Privacy')
        await page.getByRole('checkbox', { name: 'Always show remote images' }).click()

        await expect.element(page.getByRole('alert')).toHaveTextContent('disk full')
        await expect.element(page.getByRole('checkbox', { name: 'Always show remote images' })).not.toBeChecked()
        expect(state.settings.remoteImages).toBe(false)
    })

    it('shows a message as a light page in the dark theme, so its own dark style stays off', async () => {
        // The frame tells the message which scheme it is shown in; a message's
        // dark-mode rules make its text light, for a dark page it does not get.
        document.documentElement.style.colorScheme = 'dark'
        try {
            start()

            await row('Quarterly report').click()

            const frame = page.getByTitle('Message content').element()
            expect(getComputedStyle(frame).colorScheme).toBe('light')
        } finally {
            document.documentElement.style.colorScheme = ''
        }
    })

    it('opens links from a message in the system browser, not in the app', async () => {
        const state = start()

        await row('Quarterly report').click()
        const frame = page.frameLocator(page.getByTitle('Message content'))
        await frame.getByRole('link', { name: 'Open the report' }).click()

        await expect.poll(() => state.opened).toEqual(['https://example.com/report'])
    })

    it('shows mail the core fetched by itself, without a refresh', async () => {
        const state = start()
        await expect.element(row('Lunch on Friday?')).toBeInTheDocument()

        state.messages = [
            {
                id: 12,
                folderId: 1,
                uid: 12,
                subject: 'Just arrived',
                from: [{ name: 'Cid', email: 'cid@example.com' }],
                to: [],
                cc: [],
                date: 1_700_000_012,
                flags: { seen: false, flagged: false, answered: false },
                size: 100,
            },
            ...state.messages,
        ]
        announce(state, { type: 'folderChanged', accountId: 1, folderId: 1 })

        await expect.element(row('Just arrived')).toBeInTheDocument()
        await expect.element(folder('Inbox')).toHaveTextContent('2')
    })

    it('asks the system for a notification with the start of the message and the sender\'s face', async () => {
        const state = start()
        await expect.element(row('Lunch on Friday?')).toBeInTheDocument()

        announce(state, {
            type: 'newMail',
            accountId: 1,
            folderId: 1,
            account: 'john@gmail.com',
            messages: [
                {
                    id: 10,
                    subject: 'Quarterly report',
                    from: { name: 'Bob', email: 'bob@example.com' },
                    preview: 'The numbers for the third quarter are in and they look better than we expected in spring, mostly thanks to the new customers.',
                },
            ],
        })

        await expect.poll(() => state.notified.length).toBe(1)
        const [notification] = state.notified
        expect(notification?.title).toBe('Bob')
        expect(notification?.body).toBe('Quarterly report\nThe numbers for the third quarter are in and they look better than we expected in spring…')
        expect(notification?.icon.kind).toBe('image')
        expect(notification?.target).toEqual({ accountId: 1, folderId: 1, messageId: 10 })
        expect(notification?.sound).toBe(true)
    })

    it('shows the notification the way the settings say', async () => {
        const state = start()
        await expect.element(row('Lunch on Friday?')).toBeInTheDocument()
        await openSettings('Notifications')

        await settings().getByRole('button', { name: 'Customize the notification' }).click()
        await settings().getByRole('combobox', { name: 'Title' }).selectOptions('Subject')
        await settings().getByRole('combobox', { name: 'Text', exact: true }).selectOptions('Your own text…')
        const own = settings().getByRole('textbox', { name: 'Your own text' })
        await own.fill('New mail for')
        await settings().getByRole('button', { name: 'Insert Account' }).click()
        await settings().getByRole('combobox', { name: 'Picture' }).selectOptions('No picture')
        await settings().getByRole('combobox', { name: 'Stays on screen' }).selectOptions('For a set time')
        await settings().getByRole('textbox', { name: 'Seconds' }).fill('25')
        await settings().getByRole('checkbox', { name: 'Play a sound' }).click()

        // The sample in the settings shows it before any mail arrives.
        await expect.element(page.getByTestId('notification-sample')).toHaveTextContent('Lunch on Friday? New mail for you@example.com')

        announce(state, {
            type: 'newMail',
            accountId: 1,
            folderId: 1,
            account: 'john@gmail.com',
            messages: [{ id: 10, subject: 'Quarterly report', from: { name: 'Bob', email: 'bob@example.com' }, preview: null }],
        })

        await expect.poll(() => state.notified.length).toBe(1)
        expect(state.notified[0]).toEqual({
            title: 'Quarterly report',
            body: 'New mail for john@gmail.com',
            icon: { kind: 'none' },
            sound: false,
            duration: 'Custom',
            seconds: 25,
            whileFocused: false,
            target: { accountId: 1, folderId: 1, messageId: 10 },
        })
    })

    it('keeps a number that is out of range from being saved, and restores the defaults', async () => {
        const state = start()
        await openSettings('Notifications')
        await settings().getByRole('button', { name: 'Customize the notification' }).click()

        const length = settings().getByRole('textbox', { name: 'Characters of the message to show' })
        await length.fill('3')
        await expect.element(length).toHaveAttribute('aria-invalid', 'true')
        expect(state.settings.notifications.previewLength).toBe(90)
        await length.fill('40')
        expect(state.settings.notifications.previewLength).toBe(40)

        await settings().getByRole('combobox', { name: 'Title' }).selectOptions('Sender\'s address')
        await settings().getByRole('button', { name: 'Restore the defaults' }).click()

        await expect.element(settings().getByRole('combobox', { name: 'Title' })).toHaveValue('sender')
        expect(state.settings.notifications.previewLength).toBe(90)
        expect(state.settings.notifications.title).toBe('{sender}')
    })

    it('takes a picture of the user\'s own for notifications', async () => {
        const state = start()
        await openSettings('Notifications')
        await settings().getByRole('button', { name: 'Customize the notification' }).click()
        await settings().getByRole('combobox', { name: 'Picture' }).selectOptions('A picture of your own')

        const pixel = Uint8Array.from(atob('R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw=='), (character) => character.charCodeAt(0))
        await page.getByTestId('notification-picture').upload(new File([pixel], 'me.gif', { type: 'image/gif' }))

        await expect.poll(() => state.settings.notifications.customIcon?.startsWith('data:image/png;base64,')).toBe(true)
        expect(state.settings.notifications.icon).toBe('Custom')
    })

    it('opens the message when its notification is clicked', async () => {
        const state = start()
        await expect.element(folder('Inbox')).toBeInTheDocument()
        await folder('Sent').click()

        clickNotification(state, { accountId: 1, folderId: 1, messageId: 10 })

        await expect.element(page.getByRole('heading', { name: 'Quarterly report' })).toBeInTheDocument()
    })

    it('turns notifications off in the settings', async () => {
        const state = start()

        await openSettings('Notifications')
        await settings().getByRole('checkbox', { name: 'Notify about new mail' }).click()

        expect(state.settings.notifications.enabled).toBe(false)
        // The rest of it is out of the way while it is off.
        await expect.element(settings().getByRole('checkbox', { name: 'Play a sound' })).not.toBeInTheDocument()
    })

    it('shows one category of the settings at a time, chosen from the list beside it', async () => {
        start()
        await openSettings('Accounts')

        const tab = (name: string) => settings().getByRole('tab', { name })
        await expect.element(tab('Accounts')).toHaveAttribute('aria-selected', 'true')
        await expect.element(settings().getByRole('tabpanel', { name: 'Accounts' })).toBeInTheDocument()
        await expect.element(settings().getByRole('checkbox', { name: 'Show sender pictures' })).not.toBeInTheDocument()

        await tab('Privacy').click()
        await expect.element(settings().getByRole('checkbox', { name: 'Show sender pictures' })).toBeInTheDocument()
        await expect.element(settings().getByRole('button', { name: 'Add account' })).not.toBeInTheDocument()

        // The keyboard walks the list as well.
        await userEvent.keyboard('{ArrowDown}')
        await expect.element(tab('Notifications')).toHaveAttribute('aria-selected', 'true')
        await expect.element(tab('Notifications')).toHaveFocus()
        await expect.element(settings().getByRole('checkbox', { name: 'Notify about new mail' })).toBeInTheDocument()
        await userEvent.keyboard('{ArrowDown}')
        await expect.element(tab('Accounts')).toHaveAttribute('aria-selected', 'true')

        // Closed and opened again, it is where it was left.
        await tab('Privacy').click()
        await settings().getByRole('button', { name: 'Done' }).click()
        await page.getByRole('button', { name: /^Account: / }).click()
        await page.getByRole('menuitem', { name: 'Settings' }).click()
        await expect.element(tab('Privacy')).toHaveAttribute('aria-selected', 'true')
    })

    it('shows the stored theme from the start and switches it in the settings', async () => {
        const state = seededState()
        state.settings = { ...state.settings, theme: 'Slate' }
        render(Workspace, { api: createFakeApi(state) })

        const root = document.documentElement
        const token = (element: Element, name: string) => getComputedStyle(element).getPropertyValue(name).trim()
        await expect.poll(() => root.dataset['theme']).toBe('slate')
        // The shell draws its dialogs dark too, and shows the window only now.
        expect(state.colorSchemes).toEqual(['dark'])
        const dark = token(root, '--surface')

        await openSettings('Appearance')
        const choice = (name: RegExp) => settings().getByRole('radio', { name })
        await expect.element(choice(/^Dark/)).toHaveAttribute('aria-checked', 'true')
        // Each choice is drawn in its own theme, whatever the app shows now.
        const lightScene = choice(/^Light/).element().querySelector('[data-theme="aero"]')
        expect(lightScene).not.toBe(null)
        if (lightScene !== null) {
            expect(token(lightScene, '--surface')).not.toBe(dark)
        }
        // The automatic one shows both.
        expect(choice(/^Automatic/).element().querySelectorAll('[data-theme]')).toHaveLength(2)

        await choice(/^Light/).click()
        await expect.poll(() => root.dataset['theme']).toBe('aero')
        await expect.element(choice(/^Light/)).toHaveAttribute('aria-checked', 'true')
        expect(state.settings.theme).toBe('Aero')
        expect(token(root, '--surface')).not.toBe(dark)
        expect(state.colorSchemes.at(-1)).toBe('light')

        // Like any group of radio buttons, the arrow keys move the choice.
        await userEvent.keyboard('{ArrowLeft}')
        await expect.element(choice(/^Automatic/)).toHaveFocus()
        await expect.poll(() => root.hasAttribute('data-theme')).toBe(false)
        expect(state.settings.theme).toBe('System')
        expect(state.colorSchemes.at(-1)).toBe('system')
    })

    it('keeps the theme it has when the new one cannot be stored', async () => {
        const state = start({
            setSettings: async () => {
                throw { kind: 'Storage', server: null, detail: 'disk full' }
            },
        })

        await openSettings('Appearance')
        await settings().getByRole('radio', { name: /^Dark/ }).click()

        await expect.element(page.getByRole('alert')).toHaveTextContent('disk full')
        await expect.element(settings().getByRole('radio', { name: /^Automatic/ })).toHaveAttribute('aria-checked', 'true')
        expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
        expect(state.settings.theme).toBe('System')
    })
})

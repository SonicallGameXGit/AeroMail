import { describe, expect, it, vi } from 'vitest'
import { page } from 'vitest/browser'
import { render } from 'vitest-browser-svelte'
import AccountSetup from './AccountSetup.svelte'
import { createFakeApi, emptyState, fakePassword } from './fakeApi'

const typeAddress = async (email: string) => {
    await page.getByLabelText('Email address').fill(email)
}

const start = (overrides: Parameters<typeof createFakeApi>[1] = {}) => {
    const state = emptyState()
    const onadded = vi.fn()
    render(AccountSetup, { api: createFakeApi(state, overrides), onadded })

    return { state, onadded }
}

describe('AccountSetup', () => {
    it('shows the found provider with a summary, without opening the manual form', async () => {
        start()

        await typeAddress('john@gmail.com')
        await page.getByRole('button', { name: 'Continue' }).click()

        await expect.element(page.getByRole('heading', { name: 'Settings found for Gmail' })).toBeInTheDocument()
        await expect.element(page.getByText('imap.gmail.com:993')).toBeInTheDocument()
        await expect.element(page.getByText('This provider needs an app password')).toBeInTheDocument()
        await expect.element(page.getByLabelText('Server').first()).not.toBeInTheDocument()
    })

    it('connects a known provider with just the password', async () => {
        const { state, onadded } = start()

        await typeAddress('john@gmail.com')
        await page.getByRole('button', { name: 'Continue' }).click()
        await page.getByLabelText('Password', { exact: true }).fill(fakePassword)
        await page.getByRole('button', { name: 'Connect' }).click()

        await vi.waitFor(() => expect(onadded).toHaveBeenCalledOnce())
        expect(state.accounts.map((account) => account.settings.email)).toEqual(['john@gmail.com'])
    })

    it('explains a rejected password and adds nothing', async () => {
        const { state, onadded } = start()

        await typeAddress('john@gmail.com')
        await page.getByRole('button', { name: 'Continue' }).click()
        await page.getByLabelText('Password', { exact: true }).fill('wrong')
        await page.getByRole('button', { name: 'Connect' }).click()

        await expect.element(page.getByRole('alert')).toHaveTextContent(
            'Incoming mail: The server didn\'t accept the username or password.',
        )
        await expect.element(page.getByRole('alert')).toHaveTextContent('invalid credentials')
        expect(onadded).not.toHaveBeenCalled()
        expect(state.accounts).toEqual([])
    })

    it('opens the settings when the server cannot be reached', async () => {
        start({
            addAccount: async () => {
                throw { kind: 'Connect', server: 'Outgoing', detail: 'smtp.gmail.com:465: refused' }
            },
        })

        await typeAddress('john@gmail.com')
        await page.getByRole('button', { name: 'Continue' }).click()
        await page.getByLabelText('Password', { exact: true }).fill(fakePassword)
        await page.getByRole('button', { name: 'Connect' }).click()

        await expect.element(page.getByRole('alert')).toHaveTextContent('Outgoing mail: The server couldn\'t be reached')
        await expect.element(page.getByLabelText('Server').first()).toHaveValue('imap.gmail.com')
    })

    it('lets the user edit settings of a known provider', async () => {
        start()

        await typeAddress('john@gmail.com')
        await page.getByRole('button', { name: 'Continue' }).click()
        await page.getByRole('button', { name: 'Edit settings' }).click()

        await expect.element(page.getByLabelText('Server').first()).toHaveValue('imap.gmail.com')
    })

    it('opens the pre-filled manual form when the domain is unknown', async () => {
        start()

        await typeAddress('me@corp.example')
        await page.getByRole('button', { name: 'Continue' }).click()

        await expect.element(page.getByRole('heading', { name: 'We don\'t know corp.example yet' })).toBeInTheDocument()
        await expect.element(page.getByLabelText('Server').first()).toHaveValue('imap.corp.example')
    })

    it('opens the manual form for a known provider when asked to', async () => {
        start()

        await typeAddress('john@gmail.com')
        await page.getByRole('button', { name: 'Set up manually' }).click()

        await expect.element(page.getByRole('heading', { name: 'Manual setup' })).toBeInTheDocument()
        await expect.element(page.getByLabelText('Server').first()).toHaveValue('imap.gmail.com')
    })

    it('explains what is wrong with the address and stays on the first step', async () => {
        start()

        await typeAddress('john')
        await page.getByRole('button', { name: 'Continue' }).click()

        await expect.element(page.getByRole('alert')).toHaveTextContent('An email address needs an @')
        await expect.element(page.getByLabelText('Email address')).toBeInTheDocument()
    })

    it('reports invalid settings without trying to connect', async () => {
        const { onadded } = start()

        await typeAddress('me@corp.example')
        await page.getByRole('button', { name: 'Continue' }).click()
        await page.getByLabelText('Server').first().fill('')
        await page.getByLabelText('Password', { exact: true }).fill(fakePassword)
        await page.getByRole('button', { name: 'Connect' }).click()

        await expect.element(page.getByRole('alert')).toHaveTextContent('The server address is empty.')
        expect(onadded).not.toHaveBeenCalled()
    })

    it('shows a failure instead of hanging when the core cannot be reached', async () => {
        start({
            detectAccountSettings: async () => {
                throw new Error('core unavailable')
            },
        })

        await typeAddress('john@gmail.com')
        await page.getByRole('button', { name: 'Continue' }).click()

        await expect.element(page.getByRole('alert')).toHaveTextContent('Something went wrong')
    })
})

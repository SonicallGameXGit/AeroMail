import { describe, expect, it } from 'vitest'
import { page } from 'vitest/browser'
import { render } from 'vitest-browser-svelte'
import AccountSetup from './AccountSetup.svelte'
import { createFakeApi } from './fakeApi'

const typeAddress = async (email: string) => {
    await page.getByLabelText('Email address').fill(email)
}

describe('AccountSetup', () => {
    it('shows the found provider with a summary, without opening the manual form', async () => {
        render(AccountSetup, { api: createFakeApi() })

        await typeAddress('john@gmail.com')
        await page.getByRole('button', { name: 'Continue' }).click()

        await expect.element(page.getByRole('heading', { name: 'Settings found for Gmail' })).toBeInTheDocument()
        await expect.element(page.getByText('imap.gmail.com:993')).toBeInTheDocument()
        await expect.element(page.getByLabelText('Server').first()).not.toBeInTheDocument()
    })

    it('lets the user edit settings of a known provider', async () => {
        render(AccountSetup, { api: createFakeApi() })

        await typeAddress('john@gmail.com')
        await page.getByRole('button', { name: 'Continue' }).click()
        await page.getByRole('button', { name: 'Edit settings' }).click()

        await expect.element(page.getByLabelText('Server').first()).toHaveValue('imap.gmail.com')
    })

    it('opens the pre-filled manual form when the domain is unknown', async () => {
        render(AccountSetup, { api: createFakeApi() })

        await typeAddress('me@corp.example')
        await page.getByRole('button', { name: 'Continue' }).click()

        await expect.element(page.getByRole('heading', { name: 'We don\'t know corp.example yet' })).toBeInTheDocument()
        await expect.element(page.getByLabelText('Server').first()).toHaveValue('imap.corp.example')
    })

    it('opens the manual form for a known provider when asked to', async () => {
        render(AccountSetup, { api: createFakeApi() })

        await typeAddress('john@gmail.com')
        await page.getByRole('button', { name: 'Set up manually' }).click()

        await expect.element(page.getByRole('heading', { name: 'Manual setup' })).toBeInTheDocument()
        await expect.element(page.getByLabelText('Server').first()).toHaveValue('imap.gmail.com')
    })

    it('explains what is wrong with the address and stays on the first step', async () => {
        render(AccountSetup, { api: createFakeApi() })

        await typeAddress('john')
        await page.getByRole('button', { name: 'Continue' }).click()

        await expect.element(page.getByRole('alert')).toHaveTextContent('An email address needs an @')
        await expect.element(page.getByLabelText('Email address')).toBeInTheDocument()
    })

    it('reports problems found by the check and confirms valid settings', async () => {
        render(AccountSetup, { api: createFakeApi() })

        await typeAddress('me@corp.example')
        await page.getByRole('button', { name: 'Continue' }).click()
        await page.getByLabelText('Server').first().fill('')
        await page.getByRole('button', { name: 'Check settings' }).click()

        await expect.element(page.getByRole('alert')).toHaveTextContent('The server address is empty.')

        await page.getByLabelText('Server').first().fill('mail.corp.example')
        await page.getByRole('button', { name: 'Check settings' }).click()

        await expect.element(page.getByText('These settings look valid.')).toBeInTheDocument()
    })

    it('shows a failure instead of hanging when the core cannot be reached', async () => {
        const api = createFakeApi({
            detectAccountSettings: async () => {
                throw new Error('core unavailable')
            },
        })
        render(AccountSetup, { api })

        await typeAddress('john@gmail.com')
        await page.getByRole('button', { name: 'Continue' }).click()

        await expect.element(page.getByRole('alert')).toHaveTextContent('Something went wrong')
    })
})

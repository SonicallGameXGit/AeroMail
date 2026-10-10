import { describe, expect, it, vi } from 'vitest'
import { page } from 'vitest/browser'
import { render } from 'vitest-browser-svelte'
import '$lib/design/base.css'
import type { DetectResult, TypedDomain } from '$lib/api'
import AccountSetup from './AccountSetup.svelte'
import {
    createFakeApi,
    emptyState,
    fakePassword,
    fakeSignInUrl,
    gmailSignInDetection,
    pickedAddress,
} from './fakeApi'

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
    describe('while the address is typed', () => {
        const mark = () => page.getByTitle('Gmail address')
        const warning = () => page.getByText('Gmail addresses end in gmail.com. Is yours typed right?')

        it('shows Gmail\'s mark from the domain\'s name on', async () => {
            start()

            await typeAddress('john@gmai')
            await expect.element(mark()).not.toBeInTheDocument()

            await typeAddress('john@gmail')
            await expect.element(mark()).toBeInTheDocument()

            await typeAddress('john@gmail.')
            await expect.element(mark()).toBeInTheDocument()
            await expect.element(warning()).not.toBeInTheDocument()

            await typeAddress('john@gmail.com')
            await expect.element(mark()).toBeInTheDocument()
            await expect.element(warning()).not.toBeInTheDocument()
        })

        it('warns about another ending instead, and fixes it on request', async () => {
            start()

            await typeAddress('john@gmail.ru')
            await expect.element(warning()).toBeInTheDocument()
            await expect.element(mark()).not.toBeInTheDocument()

            await page.getByRole('button', { name: 'Change it to gmail.com' }).click()
            await expect.element(page.getByLabelText('Email address')).toHaveValue('john@gmail.com')
            await expect.element(mark()).toBeInTheDocument()
            await expect.element(warning()).not.toBeInTheDocument()
        })

        it('lets a warning wait, so a corrected ending never flashes it', async () => {
            start()

            await typeAddress('john@gmail.r')
            await new Promise((resolve) => setTimeout(resolve, 100))
            await expect.element(warning()).not.toBeInTheDocument()

            await typeAddress('john@gmail.com')
            await new Promise((resolve) => setTimeout(resolve, 500))
            await expect.element(warning()).not.toBeInTheDocument()
            await expect.element(mark()).toBeInTheDocument()

            await typeAddress('john@gmail.r')
            await expect.element(warning()).toBeInTheDocument()
        })

        it('goes on with a mistyped address when the user wants to', async () => {
            start()

            await typeAddress('me@gmail.ru')
            await page.getByRole('button', { name: 'Continue' }).click()

            await expect.element(page.getByRole('heading', { name: 'We don\'t know corp.example yet' })).toBeInTheDocument()
        })

        it('ignores an answer that comes after the address changed', async () => {
            const pending: { answer: ((answer: TypedDomain) => void) | null } = { answer: null }
            start({
                recognizeAddress: async (email) => {
                    if (email.endsWith('.ru')) {
                        return new Promise<TypedDomain>((resolve) => {
                            pending.answer = resolve
                        })
                    }

                    return { type: 'known', brand: 'Gmail', provider: 'Gmail' }
                },
            })

            await typeAddress('john@gmail.ru')
            await typeAddress('john@gmail.com')
            await expect.element(mark()).toBeInTheDocument()

            const late = pending.answer
            if (late === null) {
                throw new Error('the first address was never asked about')
            }
            late({ type: 'mistyped', provider: 'Gmail', suggestion: 'gmail.com' })
            await new Promise((resolve) => setTimeout(resolve, 50))
            await expect.element(warning()).not.toBeInTheDocument()
            await expect.element(mark()).toBeInTheDocument()
        })
    })

    describe('with a provider picked from the list', () => {
        const field = () => page.getByLabelText('Email address')
        const pick = async (name: string) => {
            await page.getByRole('button', { name: 'Email provider' }).click()
            await page.getByRole('menuitem', { name }).click()
        }

        it('keeps the domain fixed, so only the name is typed', async () => {
            const asked: string[] = []
            const base = createFakeApi(emptyState())
            start({
                detectAccountSettings: (email) => {
                    asked.push(email)

                    return base.detectAccountSettings(email)
                },
            })

            await typeAddress('john')
            await pick('Gmail @gmail.com')
            await expect.element(field()).toHaveValue('john')
            await expect.element(field()).toHaveAccessibleDescription('@gmail.com')
            // The mark takes the arrow's place and still opens the list.
            await expect.element(page.getByTitle('Gmail address')).toBeInTheDocument()

            await page.getByRole('button', { name: 'Continue' }).click()
            await expect.element(page.getByRole('heading', { name: 'Settings found for Gmail' })).toBeInTheDocument()
            expect(asked).toEqual(['john@gmail.com'])
        })

        it('keeps the name when another provider is picked', async () => {
            start()

            await typeAddress('john@gma')
            await pick('Outlook @outlook.com')
            await expect.element(field()).toHaveValue('john')
            await expect.element(field()).toHaveAccessibleDescription('@outlook.com')
            await expect.element(page.getByTitle('Gmail address')).not.toBeInTheDocument()
        })

        it('goes back to the whole address with None, or when one is pasted', async () => {
            start()

            await pick('Gmail @gmail.com')
            await typeAddress('john')
            await pick('None')
            await expect.element(field()).toHaveValue('john@gmail.com')

            await pick('Outlook @outlook.com')
            await typeAddress('me@example.com')
            await expect.element(field()).toHaveValue('me@example.com')
            await expect.element(field()).toHaveAccessibleDescription('')
        })
    })

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

    describe('by picking the account on the provider\'s page', () => {
        const gmail = () => page.getByRole('button', { name: 'Gmail', exact: true })

        it('adds the picked account with one click, without typing the address', async () => {
            const { state, onadded } = start()

            await expect.element(page.getByText('or using')).toBeInTheDocument()
            await gmail().click()

            await vi.waitFor(() => expect(onadded).toHaveBeenCalledOnce())
            expect(state.opened).toEqual([fakeSignInUrl])
            expect(state.accounts.map((account) => account.settings.email)).toEqual([pickedAddress])
            expect(state.accounts.map((account) => account.settings.incoming.auth)).toEqual(['OAuth2'])
        })

        it('offers nothing where the build cannot sign in there', async () => {
            start({ signInChoices: async () => [] })

            await expect.element(page.getByLabelText('Email address')).toBeInTheDocument()
            await expect.element(page.getByText('or using')).not.toBeInTheDocument()
            await expect.element(gmail()).not.toBeInTheDocument()
        })

        it('waits for the browser and can be cancelled without an error', async () => {
            let giveUp = (): void => undefined
            const cancelSignIn = vi.fn(async () => giveUp())
            const { onadded } = start({
                finishSignIn: () => new Promise((_resolve, reject) => {
                    giveUp = () => reject({ kind: 'SignIn', server: null, detail: 'the sign-in was cancelled' })
                }),
                cancelSignIn,
            })

            await gmail().click()
            await expect.element(page.getByText('Finish signing in in your browser')).toBeInTheDocument()
            await expect.element(gmail()).toBeDisabled()
            await page.getByRole('button', { name: 'Cancel sign-in' }).click()

            await expect.element(gmail()).toBeEnabled()
            await expect.element(page.getByRole('alert')).not.toBeInTheDocument()
            expect(cancelSignIn).toHaveBeenCalledWith(1)
            expect(onadded).not.toHaveBeenCalled()
        })

        it('says when the picked account is already in the app', async () => {
            const { onadded } = start({
                finishSignIn: async () => {
                    throw { kind: 'Duplicate', server: null, detail: 'this account is already added' }
                },
            })

            await gmail().click()

            await expect.element(page.getByRole('alert')).toHaveTextContent('This account is already in the app.')
            expect(onadded).not.toHaveBeenCalled()
        })
    })

    describe('on the provider\'s own page', () => {
        // A build that can sign in on Google's page.
        const detectAccountSettings = async (): Promise<DetectResult> => {
            return { ok: true, detection: gmailSignInDetection }
        }

        const reachSignIn = async () => {
            await typeAddress('john@gmail.com')
            await page.getByRole('button', { name: 'Continue' }).click()
        }

        it('signs in with one click and asks for no password', async () => {
            const { state, onadded } = start({ detectAccountSettings })

            await reachSignIn()
            await expect.element(page.getByLabelText('Password', { exact: true })).not.toBeInTheDocument()
            await expect.element(page.getByText('This provider needs an app password')).not.toBeInTheDocument()
            await page.getByRole('button', { name: 'Sign in with Gmail' }).click()

            await vi.waitFor(() => expect(onadded).toHaveBeenCalledOnce())
            expect(state.opened).toEqual([fakeSignInUrl])
            expect(state.accounts.map((account) => account.settings.incoming.auth)).toEqual(['OAuth2'])
        })

        it('waits for the browser and can be cancelled without an error', async () => {
            let giveUp = (): void => undefined
            const cancelSignIn = vi.fn(async () => giveUp())
            const { state, onadded } = start({
                detectAccountSettings,
                // The user never comes back from the browser.
                finishSignIn: () => new Promise((_resolve, reject) => {
                    giveUp = () => reject({ kind: 'SignIn', server: null, detail: 'the sign-in was cancelled' })
                }),
                cancelSignIn,
            })

            await reachSignIn()
            await page.getByRole('button', { name: 'Sign in with Gmail' }).click()
            await expect.element(page.getByText('Finish signing in in your browser')).toBeInTheDocument()
            await page.getByRole('button', { name: 'Open the page again' }).click()
            await page.getByRole('button', { name: 'Cancel sign-in' }).click()

            await expect.element(page.getByRole('button', { name: 'Sign in with Gmail' })).toBeEnabled()
            await expect.element(page.getByRole('alert')).not.toBeInTheDocument()
            expect(cancelSignIn).toHaveBeenCalledWith(1)
            expect(state.opened).toEqual([fakeSignInUrl, fakeSignInUrl])
            expect(onadded).not.toHaveBeenCalled()
        })

        it('explains a sign-in the provider did not complete', async () => {
            const { onadded } = start({
                detectAccountSettings,
                finishSignIn: async () => {
                    throw { kind: 'SignIn', server: null, detail: 'the provider answered: access_denied' }
                },
            })

            await reachSignIn()
            await page.getByRole('button', { name: 'Sign in with Gmail' }).click()

            await expect.element(page.getByRole('alert')).toHaveTextContent('The sign-in with your provider wasn\'t completed')
            await expect.element(page.getByRole('alert')).toHaveTextContent('access_denied')
            expect(onadded).not.toHaveBeenCalled()
        })

        it('keeps the app password one click away', async () => {
            const { state, onadded } = start({ detectAccountSettings })

            await reachSignIn()
            await page.getByRole('button', { name: 'Use a password instead' }).click()
            await expect.element(page.getByText('This provider needs an app password')).toBeInTheDocument()
            await page.getByLabelText('Password', { exact: true }).fill(fakePassword)
            await page.getByRole('button', { name: 'Connect' }).click()

            await vi.waitFor(() => expect(onadded).toHaveBeenCalledOnce())
            expect(state.opened).toEqual([])
            expect(state.accounts.map((account) => account.settings.outgoing.auth)).toEqual(['Password'])
        })

        it('says so when the app cannot open the provider\'s page, and still offers a password', async () => {
            start({
                detectAccountSettings: async () => ({
                    ok: true,
                    detection: { ...gmailSignInDetection, provider: 'Outlook', signInAvailable: false },
                }),
            })

            await reachSignIn()

            await expect.element(page.getByText('which the app can\'t open for it yet')).toBeInTheDocument()
            await expect.element(page.getByRole('button', { name: 'Use a password instead' })).toBeInTheDocument()
        })
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

    it('offers manual setup only once a known provider is recognized', async () => {
        start()

        await expect.element(page.getByRole('button', { name: 'Set up manually' })).not.toBeInTheDocument()

        await typeAddress('me@corp.example')
        await expect.element(page.getByRole('button', { name: 'Set up manually' })).not.toBeInTheDocument()

        await typeAddress('john@gmail.com')
        await expect.element(page.getByRole('button', { name: 'Set up manually' })).toBeEnabled()
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

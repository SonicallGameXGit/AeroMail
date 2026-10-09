import { afterEach, describe, expect, it, vi } from 'vitest'
import { desktopApi, toMailEvent } from '$lib/api'
import { gmailDetection } from '$lib/features/accounts/fakeApi'

afterEach(() => {
    vi.unstubAllGlobals()
})

describe('desktopApi', () => {
    it('hands the shell plain data even when given reactive state', async () => {
        const sent: unknown[] = []
        vi.stubGlobal('window', {
            mailclient: {
                // Like the shell: the parameters are copied, and a proxy cannot be.
                call: async (_command: string, params: unknown) => {
                    sent.push(structuredClone(params))

                    return { ok: true, value: [] }
                },
            },
        })
        // What Svelte's `$state` makes of an object.
        const reactive = new Proxy(gmailDetection.settings, {})

        await desktopApi.validateAccountSettings(reactive)

        expect(sent).toEqual([{ settings: gmailDetection.settings }])
    })
})

describe('toMailEvent', () => {
    it('accepts what the core announces', () => {
        const event = {
            type: 'newMail',
            accountId: 1,
            folderId: 2,
            account: 'me@example.com',
            messages: [{ id: 3, subject: 'Hi', from: { name: null, email: 'ann@example.com' } }],
        }

        expect(toMailEvent(event)).toEqual(event)
        expect(toMailEvent({ type: 'folderChanged', accountId: 1, folderId: 2 })).toEqual({ type: 'folderChanged', accountId: 1, folderId: 2 })
    })

    it('drops anything else', () => {
        expect(toMailEvent(null)).toBeNull()
        expect(toMailEvent({ type: 'folderChanged', accountId: '1', folderId: 2 })).toBeNull()
        expect(toMailEvent({ type: 'other', accountId: 1, folderId: 2 })).toBeNull()
        expect(toMailEvent({ type: 'newMail', accountId: 1, folderId: 2, account: 'a', messages: [{ id: 'x' }] })).toBeNull()
    })
})

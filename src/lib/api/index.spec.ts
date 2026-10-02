import { afterEach, describe, expect, it, vi } from 'vitest'
import { desktopApi } from '$lib/api'
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

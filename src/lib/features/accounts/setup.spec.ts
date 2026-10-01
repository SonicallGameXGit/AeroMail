import { describe, expect, it } from 'vitest'
import { isAddressError } from '$lib/api'
import { t } from '$lib/i18n'
import { createFakeApi, gmailDetection, guessDetection } from './fakeApi'
import { domainOf, parsePort, serverOf, switchIncomingProtocol, switchSecurity, withServer } from './setup'

const api = createFakeApi()

describe('switchIncomingProtocol', () => {
    it('uses the provider\'s own POP3 endpoint when it is known', async () => {
        const settings = await switchIncomingProtocol(api, gmailDetection.settings, gmailDetection, 'Pop3')

        expect(settings.incoming.protocol).toBe('Pop3')
        expect(settings.incoming.host).toBe('pop.gmail.com')
        expect(settings.incoming.port).toBe(995)
    })

    it('goes back to the provider\'s IMAP endpoint', async () => {
        const pop3 = await switchIncomingProtocol(api, gmailDetection.settings, gmailDetection, 'Pop3')
        const imap = await switchIncomingProtocol(api, pop3, gmailDetection, 'Imap')

        expect(imap.incoming).toEqual(gmailDetection.settings.incoming)
    })

    it('keeps the typed host and only moves the port when the endpoint is unknown', async () => {
        const typed = withServer(guessDetection.settings, 'Incoming', {
            ...guessDetection.settings.incoming,
            host: 'mail.corp.example',
        })
        const settings = await switchIncomingProtocol(api, typed, guessDetection, 'Pop3')

        expect(settings.incoming.protocol).toBe('Pop3')
        expect(settings.incoming.host).toBe('mail.corp.example')
        expect(settings.incoming.port).toBe(995)
    })

    it('keeps the username the user typed', async () => {
        const typed = withServer(gmailDetection.settings, 'Incoming', {
            ...gmailDetection.settings.incoming,
            username: 'custom',
        })
        const settings = await switchIncomingProtocol(api, typed, gmailDetection, 'Pop3')

        expect(settings.incoming.username).toBe('custom')
    })
})

describe('switchSecurity', () => {
    it('moves the port to the standard one of the chosen side only', async () => {
        const settings = await switchSecurity(api, gmailDetection.settings, 'Outgoing', 'StartTls')

        expect(serverOf(settings, 'Outgoing').security).toBe('StartTls')
        expect(serverOf(settings, 'Outgoing').port).toBe(587)
        expect(serverOf(settings, 'Incoming')).toEqual(gmailDetection.settings.incoming)
    })
})

describe('parsePort', () => {
    it('accepts ports in range and maps everything else to 0', () => {
        expect(parsePort('993')).toBe(993)
        expect(parsePort('65535')).toBe(65535)
        expect(parsePort('')).toBe(0)
        expect(parsePort('abc')).toBe(0)
        expect(parsePort('99.5')).toBe(0)
        expect(parsePort('70000')).toBe(0)
        expect(parsePort('-1')).toBe(0)
    })
})

describe('domainOf', () => {
    it('returns the part after the @', () => {
        expect(domainOf('me@corp.example')).toBe('corp.example')
        expect(domainOf('no-at')).toBe('no-at')
    })
})

describe('isAddressError', () => {
    it('recognises only the core\'s address errors', () => {
        expect(isAddressError('MissingAt')).toBe(true)
        expect(isAddressError('Nope')).toBe(false)
        expect(isAddressError({ message: 'MissingAt' })).toBe(false)
    })
})

describe('t', () => {
    it('fills in parameters', () => {
        expect(t('setup.found', { provider: 'Gmail' })).toBe('Settings found for Gmail')
    })
})

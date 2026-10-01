// In-memory `Api` for tests. It returns canned answers; the real rules are
// tested in the Rust core.

import type { Api, Detection, SettingsIssue } from '$lib/api'

export const gmailDetection: Detection = {
    source: 'BuiltIn',
    provider: 'Gmail',
    passwordKind: 'AppPassword',
    settings: {
        email: 'john@gmail.com',
        displayName: '',
        incoming: {
            protocol: 'Imap',
            host: 'imap.gmail.com',
            port: 993,
            security: 'Tls',
            auth: 'OAuth2',
            username: 'john@gmail.com',
        },
        outgoing: {
            protocol: 'Smtp',
            host: 'smtp.gmail.com',
            port: 465,
            security: 'Tls',
            auth: 'OAuth2',
            username: 'john@gmail.com',
        },
    },
    pop3: {
        protocol: 'Pop3',
        host: 'pop.gmail.com',
        port: 995,
        security: 'Tls',
        auth: 'OAuth2',
        username: 'john@gmail.com',
    },
}

export const guessDetection: Detection = {
    source: 'Guess',
    provider: null,
    passwordKind: 'Account',
    settings: {
        email: 'me@corp.example',
        displayName: '',
        incoming: {
            protocol: 'Imap',
            host: 'imap.corp.example',
            port: 993,
            security: 'Tls',
            auth: 'Password',
            username: 'me@corp.example',
        },
        outgoing: {
            protocol: 'Smtp',
            host: 'smtp.corp.example',
            port: 465,
            security: 'Tls',
            auth: 'Password',
            username: 'me@corp.example',
        },
    },
    pop3: null,
}

export const createFakeApi = (overrides: Partial<Api> = {}): Api => {
    return {
        detectAccountSettings: async (email) => {
            if (!email.includes('@')) {
                return { ok: false, error: 'MissingAt' }
            }
            if (email.endsWith('@gmail.com')) {
                return { ok: true, detection: gmailDetection }
            }

            return { ok: true, detection: guessDetection }
        },
        validateAccountSettings: async (settings) => {
            const issues: SettingsIssue[] = []
            if (settings.incoming.host === '') {
                issues.push({ server: 'Incoming', kind: 'EmptyHost', blocking: true })
            }

            return issues
        },
        defaultPort: async (protocol, security) => {
            if (protocol === 'Pop3') {
                return security === 'Tls' ? 995 : 110
            }
            if (protocol === 'Imap') {
                return security === 'Tls' ? 993 : 143
            }
            if (security === 'Tls') {
                return 465
            }

            return security === 'StartTls' ? 587 : 25
        },
        ...overrides,
    }
}

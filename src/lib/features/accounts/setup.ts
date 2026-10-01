// Presentation logic of the account setup screen. The rules themselves
// (detection, validation, default ports) live in the Rust core.

import type {
    AccountSettings,
    Api,
    AuthMethod,
    Detection,
    Protocol,
    Security,
    ServerRole,
    ServerSettings,
} from '$lib/api'

export const incomingProtocols: Protocol[] = ['Imap', 'Pop3']
export const securities: Security[] = ['Tls', 'StartTls', 'None']
export const authMethods: AuthMethod[] = ['Password', 'OAuth2']

export const serverOf = (settings: AccountSettings, role: ServerRole): ServerSettings => {
    if (role === 'Incoming') {
        return settings.incoming
    }

    return settings.outgoing
}

export const withServer = (
    settings: AccountSettings,
    role: ServerRole,
    server: ServerSettings,
): AccountSettings => {
    if (role === 'Incoming') {
        return { ...settings, incoming: server }
    }

    return { ...settings, outgoing: server }
}

// Switching the incoming protocol: take the provider's own endpoint when it is
// known, otherwise keep what the user typed and only move to the standard port.
export const switchIncomingProtocol = async (
    api: Api,
    settings: AccountSettings,
    detection: Detection,
    protocol: Protocol,
): Promise<AccountSettings> => {
    const current = settings.incoming
    if (current.protocol === protocol) {
        return settings
    }

    const known = protocol === 'Pop3' ? detection.pop3 : detection.settings.incoming
    if (known !== null && known.protocol === protocol) {
        return { ...settings, incoming: { ...known, username: current.username, auth: current.auth } }
    }

    const port = await api.defaultPort(protocol, current.security)

    return { ...settings, incoming: { ...current, protocol, port } }
}

export const switchSecurity = async (
    api: Api,
    settings: AccountSettings,
    role: ServerRole,
    security: Security,
): Promise<AccountSettings> => {
    const current = serverOf(settings, role)
    const port = await api.defaultPort(current.protocol, security)

    return withServer(settings, role, { ...current, security, port })
}

export const parsePort = (text: string): number => {
    const port = Number(text)
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
        return 0
    }

    return port
}

export const domainOf = (email: string): string => {
    const at = email.lastIndexOf('@')
    if (at === -1) {
        return email
    }

    return email.slice(at + 1)
}

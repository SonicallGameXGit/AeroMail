// The only place in the UI that talks to the Rust core (AGENTS.md 5.2).
// Components receive an `Api` and never call Tauri directly, so the same UI
// can later run against another transport or a fake in tests.

import { invoke } from '@tauri-apps/api/core'
import type { AccountSettings } from './generated/AccountSettings'
import type { AddressError } from './generated/AddressError'
import type { Detection } from './generated/Detection'
import type { Protocol } from './generated/Protocol'
import type { Security } from './generated/Security'
import type { SettingsIssue } from './generated/SettingsIssue'

export type { AccountSettings } from './generated/AccountSettings'
export type { AddressError } from './generated/AddressError'
export type { AuthMethod } from './generated/AuthMethod'
export type { Detection } from './generated/Detection'
export type { DetectionSource } from './generated/DetectionSource'
export type { IssueKind } from './generated/IssueKind'
export type { PasswordKind } from './generated/PasswordKind'
export type { Protocol } from './generated/Protocol'
export type { Security } from './generated/Security'
export type { ServerRole } from './generated/ServerRole'
export type { ServerSettings } from './generated/ServerSettings'
export type { SettingsIssue } from './generated/SettingsIssue'

export type DetectResult =
    | { ok: true, detection: Detection }
    | { ok: false, error: AddressError }

export type Api = {
    detectAccountSettings: (email: string) => Promise<DetectResult>
    validateAccountSettings: (settings: AccountSettings) => Promise<SettingsIssue[]>
    defaultPort: (protocol: Protocol, security: Security) => Promise<number>
}

const addressErrors: readonly string[] = [
    'Empty',
    'MissingAt',
    'MultipleAt',
    'EmptyLocalPart',
    'InvalidLocalPart',
    'InvalidDomain',
] satisfies AddressError[]

export const isAddressError = (value: unknown): value is AddressError => {
    return typeof value === 'string' && addressErrors.includes(value)
}

export const tauriApi: Api = {
    detectAccountSettings: async (email) => {
        try {
            const detection = await invoke<Detection>('detect_account_settings', { email })

            return { ok: true, detection }
        } catch (error) {
            if (isAddressError(error)) {
                return { ok: false, error }
            }

            throw error
        }
    },
    validateAccountSettings: (settings) => {
        return invoke<SettingsIssue[]>('validate_account_settings', { settings })
    },
    defaultPort: (protocol, security) => {
        return invoke<number>('default_port', { protocol, security })
    },
}

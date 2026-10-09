// What the desktop shell puts on `window.mailclient` (electron/preload.cts).
// Keep the two in step.

// `error` is a MailError, or an AddressError for `detect_account_settings`.
export type Reply<T> =
    | { ok: true, value: T }
    | { ok: false, error: unknown }

import type { NotificationDuration } from './generated/NotificationDuration'

// What a notification says and shows, and where a click on it leads. The
// words and pictures are the UI's; the shell only shows them.
export type NotificationRequest = {
    title: string
    body: string
    icon: NotificationIconRequest
    sound: boolean
    duration: NotificationDuration
    // How long it stays when `duration` is 'Custom'.
    seconds: number
    // Shown even while the window is in front.
    whileFocused: boolean
    target: NotificationTarget
}

// 'app' is the app's own icon; an image is a PNG `data:` address.
export type NotificationIconRequest =
    | { kind: 'app' }
    | { kind: 'none' }
    | { kind: 'image', url: string }

export type NotificationTarget = {
    accountId: number
    folderId: number
    messageId: number
}

export type DesktopBridge = {
    // One command of the mail core (crates/mail-host). `T` is the type the
    // command answers with, as generated from the Rust side.
    // `params` must be plain data: the shell copies it, and refuses proxies.
    call: <T>(command: string, params: unknown) => Promise<Reply<T>>
    // Asks where to save; the value is false when the user cancels.
    saveAttachment: (messageId: number, index: number, name: string) => Promise<Reply<boolean>>
    openExternal: (url: string) => Promise<Reply<null>>
    // Shows a system notification (not while the window is in front, unless asked).
    notify: (notification: NotificationRequest) => Promise<Reply<null>>
    // What the core announces by itself. The value is not checked: it is
    // whatever the core sent. Both return the function that stops listening.
    onEvent: (listener: (event: unknown) => void) => () => void
    onNotificationOpen: (listener: (target: unknown) => void) => () => void
}

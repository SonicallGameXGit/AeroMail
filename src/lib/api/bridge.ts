// What the desktop shell puts on `window.mailclient` (electron/preload.cts).
// Keep the two in step.

// `error` is a MailError, or an AddressError for `detect_account_settings`.
export type Reply<T> =
    | { ok: true, value: T }
    | { ok: false, error: unknown }

export type DesktopBridge = {
    // One command of the mail core (crates/mail-host). `T` is the type the
    // command answers with, as generated from the Rust side.
    call: <T>(command: string, params: Record<string, unknown>) => Promise<Reply<T>>
    // Asks where to save; the value is false when the user cancels.
    saveAttachment: (messageId: number, index: number, name: string) => Promise<Reply<boolean>>
    openExternal: (url: string) => Promise<Reply<null>>
}

// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
    namespace App {
        // interface Error {}
        // interface Locals {}
        // interface PageData {}
        // interface PageState {}
        // interface Platform {}
    }

    interface Window {
        // Present only inside the desktop app (electron/preload.cts).
        mailclient?: import('$lib/api/bridge').DesktopBridge
    }
}

export {}

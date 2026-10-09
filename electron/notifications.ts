// System notifications. Electron's `Notification` uses the native service of
// each system (Windows toasts, the macOS notification center, the freedesktop
// service on Linux); what differs between them is handled here only.

import { BrowserWindow, nativeImage, Notification, type NativeImage } from 'electron'

type Duration = 'System' | 'Custom' | 'UntilDismissed'

type Icon =
    | { kind: 'app' }
    | { kind: 'none' }
    | { kind: 'image', url: string }

export type NotificationRequest = {
    title: string
    body: string
    icon: Icon
    sound: boolean
    duration: Duration
    seconds: number
    whileFocused: boolean
    target: unknown
}

// Pictures come from the page as small PNGs; nothing bigger is accepted.
const imagePrefix = 'data:image/png;base64,'
const imageLimit = 1024 * 1024
const maxSeconds = 3600

const parseIcon = (value: unknown): Icon | null => {
    if (typeof value !== 'object' || value === null || !('kind' in value)) {
        return null
    }
    if (value.kind === 'app' || value.kind === 'none') {
        return { kind: value.kind }
    }
    if (value.kind !== 'image' || !('url' in value) || typeof value.url !== 'string') {
        return null
    }
    if (!value.url.startsWith(imagePrefix) || value.url.length > imageLimit) {
        return null
    }

    return { kind: 'image', url: value.url }
}

const parseDuration = (value: unknown): Duration | null => {
    if (value === 'System' || value === 'Custom' || value === 'UntilDismissed') {
        return value
    }

    return null
}

// Comes from the page, so it is checked like any other input.
export const parseNotification = (value: unknown): NotificationRequest | null => {
    if (typeof value !== 'object' || value === null) {
        return null
    }
    if (!('title' in value) || typeof value.title !== 'string' || !('body' in value) || typeof value.body !== 'string') {
        return null
    }
    if (!('sound' in value) || typeof value.sound !== 'boolean' || !('whileFocused' in value) || typeof value.whileFocused !== 'boolean') {
        return null
    }
    if (!('seconds' in value) || typeof value.seconds !== 'number' || !Number.isFinite(value.seconds)) {
        return null
    }
    if (!('icon' in value) || !('duration' in value) || !('target' in value)) {
        return null
    }
    const icon = parseIcon(value.icon)
    const duration = parseDuration(value.duration)
    if (icon === null || duration === null) {
        return null
    }

    return {
        // Notification services cut long texts differently; none needs more than this.
        title: value.title.slice(0, 200),
        body: value.body.slice(0, 1000),
        icon,
        sound: value.sound,
        duration,
        seconds: Math.min(Math.max(Math.round(value.seconds), 1), maxSeconds),
        whileFocused: value.whileFocused,
        target: value.target,
    }
}

const iconImage = (icon: Icon, appIcon: string): NativeImage | undefined => {
    if (icon.kind === 'none') {
        return undefined
    }
    if (icon.kind === 'image') {
        const image = nativeImage.createFromDataURL(icon.url)
        if (!image.isEmpty()) {
            return image
        }
    }

    return nativeImage.createFromPath(appIcon)
}

// Held until closed: a notification that is garbage collected loses its click.
const shown = new Set<Notification>()

// Shows the notification for `window`; a click brings the window forward
// and hands `target` to its page. Returns false when nothing was shown.
export const showNotification = (
    window: BrowserWindow,
    request: NotificationRequest,
    appIcon: string,
    onOpen: (target: unknown) => void,
): boolean => {
    // The user is looking at the list, which already updated.
    const inFront = window.isFocused() && window.isVisible() && !window.isMinimized()
    if (inFront && !request.whileFocused) {
        return false
    }
    // Some Linux systems have no notification service: that is not an error.
    if (!Notification.isSupported()) {
        return false
    }

    const notification = new Notification({
        title: request.title,
        body: request.body,
        icon: iconImage(request.icon, appIcon),
        silent: !request.sound,
        // A set time is kept by closing it ourselves, so the system must not
        // take it away first. (macOS decides by itself either way.)
        timeoutType: request.duration === 'System' ? 'default' : 'never',
    })

    let timer: NodeJS.Timeout | undefined
    const forget = () => {
        clearTimeout(timer)
        shown.delete(notification)
    }
    notification.on('click', () => {
        forget()
        if (window.isDestroyed()) {
            return
        }
        if (window.isMinimized()) {
            window.restore()
        }
        window.show()
        window.focus()
        onOpen(request.target)
    })
    notification.on('close', forget)
    notification.on('failed', forget)
    shown.add(notification)
    notification.show()

    if (request.duration === 'Custom') {
        timer = setTimeout(() => {
            notification.close()
            forget()
        }, request.seconds * 1000)
    }

    return true
}

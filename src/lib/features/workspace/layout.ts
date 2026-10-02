// Widths of the workspace panes. They are a preference of this window on
// this machine, so they are kept by the page itself, not by the core.

export type Layout = {
    sidebar: number
    list: number
}

// The sidebar at its narrowest: icons only.
export const railWidth = 52

export const sidebarRange = { min: railWidth, max: 420 }
export const listRange = { min: 240, max: 720 }
export const defaultLayout: Layout = { sidebar: 220, list: 360 }

// Narrower than this, folder names no longer fit and the sidebar becomes the rail.
const narrowestWithNames = 132

const storageKey = 'workspace.layout'

export const clampSidebar = (width: number): number => {
    if (width < narrowestWithNames) {
        return railWidth
    }

    return Math.min(Math.round(width), sidebarRange.max)
}

export const clampList = (width: number): number => {
    return Math.min(Math.max(Math.round(width), listRange.min), listRange.max)
}

// Reads what was stored; anything missing or broken falls back to the default.
export const parseLayout = (text: string | null): Layout => {
    if (text === null) {
        return defaultLayout
    }

    let stored: unknown
    try {
        stored = JSON.parse(text)
    } catch {
        return defaultLayout
    }
    if (typeof stored !== 'object' || stored === null) {
        return defaultLayout
    }

    const sidebar = 'sidebar' in stored && typeof stored.sidebar === 'number' && Number.isFinite(stored.sidebar)
        ? clampSidebar(stored.sidebar)
        : defaultLayout.sidebar
    const list = 'list' in stored && typeof stored.list === 'number' && Number.isFinite(stored.list)
        ? clampList(stored.list)
        : defaultLayout.list

    return { sidebar, list }
}

// Storage can be unavailable (private mode, a full disk); the layout then just lasts for the session.
export const loadLayout = (): Layout => {
    try {
        return parseLayout(localStorage.getItem(storageKey))
    } catch {
        return defaultLayout
    }
}

export const saveLayout = (layout: Layout) => {
    try {
        localStorage.setItem(storageKey, JSON.stringify(layout))
    } catch {
        return
    }
}

// The themes the app can be shown in. tokens.css draws each one under its
// `data-theme`; this says which attribute a theme is and whether it is light
// or dark, for the shell's own parts.

import type { ColorScheme, Theme } from '$lib/api'

type Look = {
    // The `data-theme` of tokens.css; null follows the system.
    attribute: string | null
    scheme: ColorScheme
}

// A record, so a theme added to the core cannot be forgotten here.
const looks: Record<Theme, Look> = {
    System: { attribute: null, scheme: 'system' },
    Aero: { attribute: 'aero', scheme: 'light' },
    Slate: { attribute: 'slate', scheme: 'dark' },
}

// In the order the settings offer them.
export const themes: Theme[] = [
    'System',
    'Aero',
    'Slate',
]

export const themeAttribute = (theme: Theme): string | null => {
    return looks[theme].attribute
}

export const colorScheme = (theme: Theme): ColorScheme => {
    return looks[theme].scheme
}

// Everything inside `element` is drawn in `theme`.
export const applyTheme = (element: HTMLElement, theme: Theme) => {
    const attribute = themeAttribute(theme)
    if (attribute === null) {
        element.removeAttribute('data-theme')
        return
    }

    element.setAttribute('data-theme', attribute)
}

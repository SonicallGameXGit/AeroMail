import { describe, expect, it } from 'vitest'
import { colorScheme, themeAttribute, themes } from './theme'

describe('themes', () => {
    it('follows the system without a data-theme of its own', () => {
        expect(themeAttribute('System')).toBe(null)
        expect(colorScheme('System')).toBe('system')
    })

    it('draws Aero light and Slate dark, under the names tokens.css uses', () => {
        expect(themeAttribute('Aero')).toBe('aero')
        expect(colorScheme('Aero')).toBe('light')
        expect(themeAttribute('Slate')).toBe('slate')
        expect(colorScheme('Slate')).toBe('dark')
    })

    it('offers every theme once, the system first', () => {
        expect(themes[0]).toBe('System')
        expect(new Set(themes).size).toBe(themes.length)
    })
})

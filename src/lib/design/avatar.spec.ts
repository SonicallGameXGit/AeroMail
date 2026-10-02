import { describe, expect, it } from 'vitest'
import { avatarInitial, avatarTone, avatarTones } from './avatar'

describe('avatar', () => {
    it('takes the first letter or digit of a name', () => {
        expect(avatarInitial('ann')).toBe('A')
        expect(avatarInitial(' "Bob" Stone')).toBe('B')
        expect(avatarInitial('x-kom')).toBe('X')
        expect(avatarInitial('żaneta')).toBe('Ż')
        expect(avatarInitial('42 Ltd')).toBe('4')
        expect(avatarInitial('***')).toBe('?')
        expect(avatarInitial('')).toBe('?')
    })

    it('gives an address the same colour every time, whatever its case', () => {
        expect(avatarTone('ann@example.com')).toBe(avatarTone(' Ann@Example.com '))

        for (const seed of ['', 'a', 'ann@example.com', 'bob@example.com', 'żaneta@example.pl']) {
            expect(avatarTone(seed)).toBeGreaterThanOrEqual(1)
            expect(avatarTone(seed)).toBeLessThanOrEqual(avatarTones)
        }
    })

    it('spreads different addresses over several colours', () => {
        const seeds = Array.from({ length: 40 }, (_, index) => `person${index}@example.com`)

        expect(new Set(seeds.map(avatarTone)).size).toBeGreaterThan(3)
    })
})

import { describe, expect, it } from 'vitest'
import { clampList, clampSidebar, defaultLayout, listRange, parseLayout, railWidth, sidebarRange } from './layout'

describe('pane widths', () => {
    it('snaps a sidebar too narrow for names to the icon rail', () => {
        expect(clampSidebar(0)).toBe(railWidth)
        expect(clampSidebar(120)).toBe(railWidth)
        expect(clampSidebar(180.4)).toBe(180)
        expect(clampSidebar(5000)).toBe(sidebarRange.max)
    })

    it('keeps the message list within its range', () => {
        expect(clampList(10)).toBe(listRange.min)
        expect(clampList(400.6)).toBe(401)
        expect(clampList(5000)).toBe(listRange.max)
    })

    it('restores stored widths and survives anything else', () => {
        expect(parseLayout('{ "sidebar": 300, "list": 500 }')).toEqual({ sidebar: 300, list: 500 })
        expect(parseLayout('{ "sidebar": 9999, "list": 1 }')).toEqual({ sidebar: sidebarRange.max, list: listRange.min })
        expect(parseLayout('{ "sidebar": "wide" }')).toEqual(defaultLayout)
        expect(parseLayout('[1, 2]')).toEqual(defaultLayout)
        expect(parseLayout('not json')).toEqual(defaultLayout)
        expect(parseLayout('null')).toEqual(defaultLayout)
        expect(parseLayout(null)).toEqual(defaultLayout)
    })
})

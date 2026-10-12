import { describe, expect, it } from 'vitest'
import { anchorAt, emptySelection, extend, inOrder, keepListed, selectAll, toggle } from './selection'

const order = [
    50,
    40,
    30,
    20,
    10,
]

describe('selection', () => {
    it('ticks and unticks one row at a time', () => {
        const one = toggle(emptySelection, 30)
        expect(one.keys).toEqual([30])
        expect(one.anchor).toBe(30)

        const two = toggle(one, 10)
        expect(two.keys).toEqual([30, 10])
        expect(toggle(two, 30).keys).toEqual([10])
    })

    it('ticks a range from the anchor, downwards and upwards', () => {
        const start = anchorAt(emptySelection, 40)

        expect(extend(start, order, 20).keys).toEqual([40, 30, 20])
        expect(extend(start, order, 50).keys).toEqual([50, 40])
    })

    it('shrinks a range walked back over, keeping what was ticked before', () => {
        const before = toggle(emptySelection, 10)
        const start = anchorAt(before, 50)
        const wide = extend(start, order, 30)
        expect(wide.keys).toEqual([10, 50, 40, 30])

        const narrow = extend(wide, order, 40)
        expect(narrow.keys).toEqual([10, 50, 40])
        expect(narrow.anchor).toBe(50)
    })

    it('starts a range at the row itself without an anchor', () => {
        expect(extend(emptySelection, order, 20).keys).toEqual([20])
    })

    it('selects every row and forgets rows that left the list', () => {
        const all = selectAll(order)
        expect(all.keys).toEqual(order)

        const kept = keepListed(all, [50, 30])
        expect(kept.keys).toEqual([50, 30])
        expect(kept.anchor).toBe(50)
        expect(keepListed(toggle(emptySelection, 40), [50]).anchor).toBeNull()
    })

    it('lists the ticked rows in the list\'s order', () => {
        const ticked = toggle(toggle(emptySelection, 10), 50)

        expect(inOrder(ticked, order)).toEqual([50, 10])
    })
})

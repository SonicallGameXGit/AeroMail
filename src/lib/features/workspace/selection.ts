// Which rows of the message list are ticked, the way a file manager selects:
// one at a time, or a range from an anchor. Rows are named by the id of
// their newest message (`Conversation.latest.id`).

export type Selection = {
    keys: number[]
    // Where a range starts: the row last clicked or ticked, or walked to
    // without Shift.
    anchor: number | null
    // What was ticked when the anchor was set. A range is added to it, so
    // walking back with Shift shrinks the range without losing earlier ticks.
    base: number[]
}

export const emptySelection: Selection = {
    keys: [],
    anchor: null,
    base: [],
}

// Ticks or unticks one row; it becomes the anchor.
export const toggle = (selection: Selection, key: number): Selection => {
    const keys = selection.keys.includes(key)
        ? selection.keys.filter((candidate) => candidate !== key)
        : [...selection.keys, key]

    return { keys, anchor: key, base: keys }
}

// Moves the anchor without ticking anything.
export const anchorAt = (selection: Selection, key: number): Selection => {
    return { ...selection, anchor: key, base: selection.keys }
}

// Ticks every row from the anchor to `key` (in the list's `order`), on top of
// what was ticked before the anchor was set. Without an anchor, `key` is one.
export const extend = (selection: Selection, order: number[], key: number): Selection => {
    const anchor = selection.anchor !== null && order.includes(selection.anchor) ? selection.anchor : key
    const from = order.indexOf(anchor)
    const to = order.indexOf(key)
    if (to === -1) {
        return selection
    }
    const range = order.slice(Math.min(from, to), Math.max(from, to) + 1)
    const keys = [...selection.base.filter((candidate) => !range.includes(candidate)), ...range]

    return { keys, anchor, base: selection.base }
}

export const selectAll = (order: number[]): Selection => {
    return { keys: [...order], anchor: order[0] ?? null, base: [...order] }
}

// Forgets rows that are no longer listed (moved, deleted, another tab).
export const keepListed = (selection: Selection, order: number[]): Selection => {
    const listed = (key: number) => order.includes(key)
    const keys = selection.keys.filter(listed)
    if (keys.length === selection.keys.length && (selection.anchor === null || listed(selection.anchor))) {
        return selection
    }

    return {
        keys,
        anchor: selection.anchor !== null && listed(selection.anchor) ? selection.anchor : null,
        base: selection.base.filter(listed),
    }
}

// The ticked rows in the list's order.
export const inOrder = (selection: Selection, order: number[]): number[] => {
    return order.filter((key) => selection.keys.includes(key))
}

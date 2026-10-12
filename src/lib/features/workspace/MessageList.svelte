<script lang="ts">
    import type { Category, Conversation, Folder } from '$lib/api'
    import Avatar from '$lib/design/Avatar.svelte'
    import { avatarTone } from '$lib/design/avatar'
    import Button from '$lib/design/Button.svelte'
    import Icon from '$lib/design/Icon.svelte'
    import Tick from '$lib/design/Tick.svelte'
    import { t } from '$lib/i18n'
    import ListTools from './ListTools.svelte'
    import { anchorAt, emptySelection, extend, inOrder, selectAll, toggle, type Selection } from './selection'
    import {
        categoryIcon,
        dropAction,
        folderIcon,
        folderLabel,
        participantsLabel,
        senderLabel,
        senderPicture,
        shortDate,
        subjectLabel,
        type ListAction,
        type Pictures,
    } from './workspace'

    type Props = {
        // The folder whose messages are listed.
        folder: Folder | null
        // One row each: a conversation, or a message on its own; in a sorted
        // inbox only the rows of the open tab.
        conversations: Conversation[]
        // The tabs of a sorted inbox, with what is unread in each; none
        // elsewhere.
        tabs: { category: Category, unread: number }[]
        // The open tab.
        category: Category
        // The open message; its row is the selected one.
        selectedId: number | null
        // The ticked rows.
        selection: Selection
        // The account's address, named "me" among the people of a conversation.
        ownEmail: string
        pictures: Pictures
        // True when the server has older messages than the ones shown.
        hasMore: boolean
        busy: boolean
        // What the folder lets rows do.
        canArchive: boolean
        spam: 'report' | 'undo' | null
        moveTargets: Folder[]
        onselect: (conversation: Conversation) => void
        onselection: (selection: Selection) => void
        onact: (action: ListAction, rows: Conversation[]) => void
        // The drop target under dragged rows, as its `data-drop` names it.
        ondragover: (target: string | null) => void
        oncategory: (category: Category) => void
        onmore: () => void
        onrefresh: () => void
    }

    let {
        folder,
        conversations,
        tabs,
        category,
        selectedId,
        selection,
        ownEmail,
        pictures,
        hasMore,
        busy,
        canArchive,
        spam,
        moveTargets,
        onselect,
        onselection,
        onact,
        ondragover,
        oncategory,
        onmore,
        onrefresh,
    }: Props = $props()

    const id = $props.id()
    const tabButtons: Partial<Record<Category, HTMLButtonElement>> = {}
    const rowButtons: Record<number, HTMLButtonElement | null> = {}

    let order = $derived(conversations.map((conversation) => conversation.latest.id))
    let ticked = $derived(inOrder(selection, order))
    let tickedRows = $derived(conversations.filter((conversation) => selection.keys.includes(conversation.latest.id)))

    // The picture behind a conversation's picture has the colour of someone
    // else who wrote in it, so the pile reads as more than one message.
    const pileTone = (conversation: Conversation): string => {
        const latest = conversation.latest.from[0]?.email.toLowerCase() ?? ''
        const other = conversation.senders.find((sender) => sender.email.toLowerCase() !== latest)

        return `var(--avatar-${avatarTone(other?.email ?? latest)})`
    }

    const now = new Date()

    const isSelected = (conversation: Conversation): boolean => {
        return selectedId !== null && conversation.messageIds.includes(selectedId)
    }

    // The row the keyboard is on: the one walked to last, else the open one,
    // else the first. Only it is reached with Tab; arrows walk from it.
    let walked = $state<number | null>(null)
    let focusKey = $derived.by(() => {
        if (walked !== null && order.includes(walked)) {
            return walked
        }

        return conversations.find(isSelected)?.latest.id ?? order[0] ?? null
    })

    // Puts the keyboard on a row (by default the one it was on), scrolling
    // only as far as needed to show it.
    export const focusRow = (key: number | null = focusKey) => {
        if (key === null) {
            return
        }
        walked = key
        const button = rowButtons[key]
        if (button !== undefined && button !== null) {
            button.focus({ preventScroll: true })
            button.scrollIntoView({ block: 'nearest' })
        }
    }

    // Left and right walk the tabs, Home and End jump to the ends, as in any
    // row of tabs; a tab opens as soon as it is reached.
    const walkTabs = (event: KeyboardEvent, index: number) => {
        const last = tabs.length - 1
        const moves: Record<string, number> = {
            ArrowRight: index === last ? 0 : index + 1,
            ArrowLeft: index === 0 ? last : index - 1,
            Home: 0,
            End: last,
        }
        const target = tabs[moves[event.key] ?? -1]
        if (target === undefined) {
            return
        }

        event.preventDefault()
        oncategory(target.category)
        tabButtons[target.category]?.focus()
    }

    // What a key or a tool acts on: the ticked rows, else the row itself.
    const targets = (conversation: Conversation): Conversation[] => {
        return tickedRows.length > 0 ? tickedRows : [conversation]
    }

    const starAll = (rows: Conversation[]) => {
        onact({ kind: 'star', star: !rows.every((row) => row.flagged) }, rows)
    }

    // A file manager's keys: arrows walk, Shift with them ticks a range,
    // Space ticks one, Enter opens. Shift+Left deletes and Shift+Right stars,
    // like swiping a row on a phone.
    const rowKey = (event: KeyboardEvent, index: number) => {
        const conversation = conversations[index]
        if (conversation === undefined) {
            return
        }
        const key = conversation.latest.id
        const moves: Record<string, number> = {
            ArrowDown: Math.min(index + 1, order.length - 1),
            ArrowUp: Math.max(index - 1, 0),
            Home: 0,
            End: order.length - 1,
        }
        const move = moves[event.key]
        if (move !== undefined && !event.ctrlKey && !event.altKey && !event.metaKey) {
            event.preventDefault()
            const next = order[move]
            if (next === undefined) {
                return
            }
            onselection(event.shiftKey ? extend(selection, order, next) : anchorAt(selection, next))
            focusRow(next)

            return
        }

        if (event.key === ' ') {
            event.preventDefault()
            onselection(toggle(selection, key))
        } else if (event.key === 'ArrowLeft' && event.shiftKey) {
            event.preventDefault()
            onact({ kind: 'delete' }, targets(conversation))
        } else if (event.key === 'ArrowRight' && event.shiftKey) {
            event.preventDefault()
            starAll(targets(conversation))
        } else if (event.key === 'Delete') {
            event.preventDefault()
            onact({ kind: 'delete' }, targets(conversation))
        } else if (event.key === 'Escape' && selection.keys.length > 0) {
            event.preventDefault()
            onselection(emptySelection)
        } else if (event.key === 'a' && (event.ctrlKey || event.metaKey)) {
            event.preventDefault()
            onselection(selectAll(order))
        }
    }

    // A button would also click on Space; here Space ticks the row instead.
    const rowKeyUp = (event: KeyboardEvent) => {
        if (event.key === ' ') {
            event.preventDefault()
        }
    }

    // Clicking opens; Ctrl adds the row to the ticked ones, Shift ticks the
    // range from the anchor, neither of them opening anything.
    const rowClick = (event: MouseEvent, conversation: Conversation) => {
        if (dropped) {
            return
        }
        const key = conversation.latest.id
        walked = key
        if (event.ctrlKey || event.metaKey) {
            onselection(toggle(selection, key))
        } else if (event.shiftKey) {
            onselection(extend(selection, order, key))
        } else {
            onselection(anchorAt(selection, key))
            onselect(conversation)
        }
    }

    const tickClick = (event: MouseEvent, conversation: Conversation) => {
        const key = conversation.latest.id
        walked = key
        onselection(event.shiftKey ? extend(selection, order, key) : toggle(selection, key))
    }

    // Dragging: rows are picked up after the pointer moved a little, and
    // drawn under it where they were grabbed. Their place in the list stays
    // empty until they are dropped on a folder or a tab, or let go. Over a
    // place that takes them they shrink, and dropped there they fall into it.

    // How far the pointer moves before a press becomes a drag.
    const dragStart = 5
    // How long the fall into a folder or tab is shown; a little longer than
    // the motion it plays (`--motion-soft`).
    const landMs = 260

    type Press = {
        conversation: Conversation
        row: HTMLElement
        pointer: number
        x: number
        y: number
    }

    type Drag = {
        rows: Conversation[]
        // The grabbed row, drawn under the pointer.
        lead: Conversation
        // Where in the row it was grabbed.
        dx: number
        dy: number
        width: number
        x: number
        y: number
    }

    let pressed: Press | null = null
    let drag = $state<Drag | null>(null)
    let over = $state<string | null>(null)
    // Rows just dropped, falling into the target: from where they were let
    // go, by how much to reach the middle of the target.
    // Raw, so the timer that ends a fall can tell it from a later one.
    let landing = $state.raw<{ from: Drag, dx: number, dy: number } | null>(null)
    let ghost = $derived(drag ?? landing?.from ?? null)
    // A drop ends in a click on the row it started from; that click opens nothing.
    let dropped = false

    let lifted = $derived(ghost?.rows.map((row) => row.latest.id) ?? [])

    // The target under the pointer, if it takes these rows: another folder,
    // or another tab of the sorted inbox.
    const targetAt = (x: number, y: number): string | null => {
        const element = document.elementFromPoint(x, y)
        const found = element instanceof Element ? element.closest('[data-drop]') : null
        const target = found?.getAttribute('data-drop') ?? null
        if (target === null) {
            return null
        }
        const action = dropAction(target)
        if (action?.kind === 'move' && action.folderId !== folder?.id) {
            return target
        }
        if (action?.kind === 'category' && action.category !== category) {
            return target
        }

        return null
    }

    const hover = (target: string | null) => {
        if (target !== over) {
            over = target
            ondragover(target)
        }
    }

    const press = (event: PointerEvent, conversation: Conversation) => {
        const start = event.target instanceof Element ? event.target : null
        if (event.button !== 0 || event.shiftKey || event.ctrlKey || event.metaKey || start?.closest('[data-nodrag]') !== null) {
            return
        }
        if (!(event.currentTarget instanceof HTMLElement)) {
            return
        }
        landing = null
        pressed = { conversation, row: event.currentTarget, pointer: event.pointerId, x: event.clientX, y: event.clientY }
    }

    const move = (event: PointerEvent) => {
        if (pressed === null || event.pointerId !== pressed.pointer) {
            return
        }
        if (drag === null) {
            if (Math.hypot(event.clientX - pressed.x, event.clientY - pressed.y) < dragStart) {
                return
            }
            const rect = pressed.row.getBoundingClientRect()
            const grabbed = pressed.conversation
            // A ticked row brings every ticked row; another one comes alone.
            const rows = selection.keys.includes(grabbed.latest.id) ? tickedRows : [grabbed]
            drag = {
                rows,
                lead: grabbed,
                dx: pressed.x - rect.left,
                dy: pressed.y - rect.top,
                width: rect.width,
                x: event.clientX,
                y: event.clientY,
            }
            pressed.row.setPointerCapture(event.pointerId)
        }
        drag = { ...drag, x: event.clientX, y: event.clientY }
        hover(targetAt(event.clientX, event.clientY))
    }

    // The grabbed row is drawn in the browser's top layer, like the menus:
    // nothing in the window can clip it or shift where it is drawn.
    const raise = (node: HTMLElement) => {
        node.showPopover()
    }

    const finish = (drop: boolean) => {
        const done = drag
        const target = over
        if (pressed !== null && pressed.row.hasPointerCapture(pressed.pointer)) {
            pressed.row.releasePointerCapture(pressed.pointer)
        }
        pressed = null
        drag = null
        hover(null)
        if (done === null) {
            return
        }

        dropped = true
        setTimeout(() => dropped = false)
        const action = target === null ? null : dropAction(target)
        if (!drop || action === null || target === null) {
            return
        }

        const place = document.querySelector(`[data-drop="${target}"]`)?.getBoundingClientRect()
        if (place !== undefined) {
            const fall = { from: done, dx: place.left + place.width / 2 - done.x, dy: place.top + place.height / 2 - done.y }
            landing = fall
            setTimeout(() => {
                if (landing === fall) {
                    landing = null
                }
            }, landMs)
        }
        onact(action, done.rows)
    }
</script>

<svelte:window
    onpointermove={move}
    onpointerup={() => finish(true)}
    onpointercancel={() => finish(false)}
    onkeydown={(event) => {
        if (event.key === 'Escape' && drag !== null) {
            event.preventDefault()
            finish(false)
        }
    }}
/>

{#snippet face(conversation: Conversation)}
    {@const message = conversation.latest}
    {@const grouped = conversation.count > 1}
    <span class="mark">
        {#if conversation.unread > 0}
            <span class="dot" role="img" aria-label={t('list.unread')}></span>
        {/if}
    </span>
    <!-- A conversation is a small pile: one picture with the edge of another behind it. -->
    <span class="picture" class:pile={grouped} style:--pile={grouped ? pileTone(conversation) : undefined}>
        <Avatar
            name={senderLabel(message)}
            seed={message.from[0]?.email ?? ''}
            image={senderPicture(pictures, message)}
        />
    </span>
    <span class="lines">
        <span class="top">
            <span class="sender">{participantsLabel(conversation, ownEmail)}</span>
            {#if grouped}
                <span class="count" aria-hidden="true">{conversation.count}</span>
                <span class="hidden">{t('list.inConversation', { count: String(conversation.count) })}</span>
            {/if}
            <span class="date">{shortDate(message.date, now)}</span>
        </span>
        <span class="subject">{subjectLabel(message)}</span>
    </span>
{/snippet}

{#snippet star(conversation: Conversation, tabindex: number | undefined)}
    <button
        class="star"
        type="button"
        {tabindex}
        title={conversation.flagged ? t('toolbar.unstar') : t('toolbar.star')}
        aria-label={conversation.flagged ? t('toolbar.unstar') : t('toolbar.star')}
        aria-pressed={conversation.flagged}
        onclick={() => onact({ kind: 'star', star: !conversation.flagged }, [conversation])}
    >
        <Icon name={conversation.flagged ? 'star' : 'starOff'} size={18} />
    </button>
{/snippet}

<section class="list" class:dragging={drag !== null}>
    <header>
        {#if conversations.length > 0}
            <span data-nodrag>
                <Tick
                    label={t('list.selectAll')}
                    checked={ticked.length > 0 && ticked.length === order.length}
                    mixed={ticked.length > 0 && ticked.length < order.length}
                    onclick={() => onselection(ticked.length === order.length ? emptySelection : selectAll(order))}
                />
            </span>
        {/if}
        {#if ticked.length > 0}
            <span class="chosen" role="status">{t('list.selected', { count: String(ticked.length) })}</span>
            <span class="spacer"></span>
            <ListTools
                seen={tickedRows.every((row) => row.unread === 0)}
                starred={tickedRows.every((row) => row.flagged)}
                {canArchive}
                {spam}
                full
                {moveTargets}
                label={t('list.selection')}
                onact={(action) => onact(action, tickedRows)}
            />
            <Button variant="tool" icon="close" label={t('list.clearSelection')} onclick={() => onselection(emptySelection)} />
        {:else}
            {#if folder !== null}
                <Icon name={folderIcon(folder)} size={20} />
                <h2>{folderLabel(folder)}</h2>
                {#if folder.unread > 0}
                    <span class="unread-count">{t('list.unreadCount', { count: String(folder.unread) })}</span>
                {/if}
            {/if}
            <span class="spacer"></span>
            <span class="refresh" class:busy>
                <Button
                    variant="tool"
                    icon="refresh"
                    label={t('toolbar.refresh')}
                    disabled={busy || folder === null}
                    onclick={onrefresh}
                />
            </span>
        {/if}
    </header>

    {#if tabs.length > 0}
        <div class="tabs" role="tablist" aria-label={t('list.categories')}>
            {#each tabs as tab, index (tab.category)}
                {@const name = t(`category.${tab.category}`)}
                <button
                    bind:this={tabButtons[tab.category]}
                    class="tab"
                    class:selected={tab.category === category}
                    class:over={over === 'category:' + tab.category}
                    type="button"
                    role="tab"
                    id={`${id}-tab-${tab.category}`}
                    data-drop={'category:' + tab.category}
                    aria-selected={tab.category === category}
                    aria-controls={`${id}-rows`}
                    aria-label={tab.unread > 0 ? t('list.categoryUnread', { name, count: String(tab.unread) }) : name}
                    title={t(`categoryHint.${tab.category}`)}
                    tabindex={tab.category === category ? 0 : -1}
                    onclick={() => oncategory(tab.category)}
                    onkeydown={(event) => walkTabs(event, index)}
                >
                    <Icon name={categoryIcon(tab.category)} size={16} />
                    <span class="name">{name}</span>
                    {#if tab.unread > 0}
                        <span class="badge" aria-hidden="true">{tab.unread}</span>
                    {/if}
                </button>
            {/each}
        </div>
    {/if}

    <div
        class="rows"
        id={`${id}-rows`}
        role={tabs.length > 0 ? 'tabpanel' : undefined}
        aria-labelledby={tabs.length > 0 ? `${id}-tab-${category}` : undefined}
    >
        {#if conversations.length === 0}
            <p class="empty">{busy ? t('app.loading') : t('list.empty')}</p>
        {:else}
            <!-- Pictures in a row are not dragged on their own: the row is. -->
            <ul aria-label={t('list.messages')} ondragstart={(event) => event.preventDefault()}>
                {#each conversations as conversation, index (conversation.latest.id)}
                    {@const key = conversation.latest.id}
                    {@const checked = selection.keys.includes(key)}
                    <li
                        class="row"
                        class:selected={isSelected(conversation)}
                        class:ticked={checked}
                        class:unread={conversation.unread > 0}
                        class:lifted={lifted.includes(key)}
                        onpointerdown={(event) => press(event, conversation)}
                    >
                        <span class="lead" data-nodrag>
                            <Tick label={t('list.select')} {checked} tabindex={-1} onclick={(event) => tickClick(event, conversation)} />
                            {@render star(conversation, -1)}
                        </span>
                        <button
                            bind:this={rowButtons[key]}
                            class="open"
                            type="button"
                            tabindex={key === focusKey ? 0 : -1}
                            aria-current={isSelected(conversation)}
                            onclick={(event) => rowClick(event, conversation)}
                            onkeydown={(event) => rowKey(event, index)}
                            onkeyup={rowKeyUp}
                            onfocus={() => walked = key}
                        >
                            {@render face(conversation)}
                        </button>
                        <span class="tools" data-nodrag>
                            <ListTools
                                seen={conversation.unread === 0}
                                starred={conversation.flagged}
                                {canArchive}
                                {spam}
                                tabindex={-1}
                                onact={(action) => onact(action, [conversation])}
                            />
                        </span>
                    </li>
                {/each}
            </ul>

            {#if hasMore}
                <div class="more">
                    <Button disabled={busy} onclick={onmore}>{t('list.loadMore')}</Button>
                </div>
            {/if}
        {/if}
    </div>
</section>

<!-- The grabbed row, looking as it did in the list, under the pointer where it was grabbed. -->
{#if ghost !== null}
    <!-- It shrinks and falls around the point it is held by, so that point stays under the pointer. -->
    <div
        class="ghost"
        popover="manual"
        use:raise
        class:stack={ghost.rows.length > 1}
        class:over={drag !== null && over !== null}
        class:landing={landing !== null}
        style:left={ghost.x - ghost.dx + 'px'}
        style:top={ghost.y - ghost.dy + 'px'}
        style:width={ghost.width + 'px'}
        style:transform-origin={`${ghost.dx}px ${ghost.dy}px`}
        style:--fall-x={(landing?.dx ?? 0) + 'px'}
        style:--fall-y={(landing?.dy ?? 0) + 'px'}
        aria-hidden="true"
    >
        <div class="row" class:unread={ghost.lead.unread > 0} class:ticked={selection.keys.includes(ghost.lead.latest.id)}>
            <span class="lead">
                <Tick label={t('list.select')} checked={selection.keys.includes(ghost.lead.latest.id)} tabindex={-1} onclick={() => undefined} />
                {@render star(ghost.lead, -1)}
            </span>
            <span class="open">
                {@render face(ghost.lead)}
            </span>
        </div>
        {#if ghost.rows.length > 1}
            <span class="carried">{t('list.dragging', { count: String(ghost.rows.length) })}</span>
        {/if}
    </div>
{/if}


<style>
    .list {
        display: flex;
        flex-direction: column;
        min-width: 0;
        min-height: 0;
        background: var(--surface);
        color: var(--text);
    }

    .dragging,
    .dragging * {
        cursor: grabbing;
    }

    header {
        flex: none;
        display: flex;
        align-items: center;
        gap: var(--space-2);
        min-height: 40px;
        padding: var(--space-1) var(--space-1) var(--space-1) var(--space-2);
        border-bottom: 1px solid var(--toolbar-border);
        background: var(--toolbar-face);
    }

    h2 {
        min-width: 0;
        overflow: hidden;
        font-size: var(--font-size-body);
        font-weight: 600;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .unread-count,
    .chosen {
        flex: none;
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    .chosen {
        font-weight: 600;
        color: var(--text);
    }

    .spacer {
        flex: 1;
    }

    .refresh {
        display: inline-flex;
    }

    .refresh.busy :global(svg) {
        animation: spin var(--motion-spin) linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    /* One row of tabs; each gives way evenly when the list is narrow, the
       picture staying and the name shortening. */
    .tabs {
        flex: none;
        display: flex;
        gap: 2px;
        padding: var(--space-1) var(--space-1) 0;
        border-bottom: 1px solid var(--toolbar-border);
        background: var(--toolbar-face);
    }

    .tab {
        flex: 0 1 auto;
        display: flex;
        align-items: center;
        gap: var(--space-1);
        min-width: 0;
        margin-bottom: -1px;
        padding: var(--space-1) var(--space-2);
        border: 1px solid transparent;
        border-radius: var(--radius-control) var(--radius-control) 0 0;
        background: none;
        color: var(--text-muted);
        cursor: default;
        transition: box-shadow var(--motion-fast);
    }

    .tab:hover {
        border-color: var(--row-hover-border);
        background: var(--row-hover-face);
        color: var(--text);
    }

    .tab.selected {
        border-color: var(--toolbar-border);
        border-bottom-color: var(--surface);
        background: var(--surface);
        color: var(--text);
        font-weight: 600;
    }

    /* A tab dragged rows would go to glows, like a selected row. */
    .tab.over {
        border-color: var(--row-selected-border);
        background: var(--row-selected-face);
        color: var(--text);
        box-shadow: var(--focus-glow);
    }

    .tab:focus-visible {
        outline: none;
        box-shadow: inset var(--focus-glow);
    }

    .name {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .badge {
        flex: none;
        padding: 0 var(--space-1);
        border-radius: var(--radius-control);
        background: var(--badge-quiet-face);
        font-size: var(--font-size-small);
        font-weight: 600;
        color: var(--text);
    }

    .rows {
        flex: 1;
        min-height: 0;
        overflow-y: auto;
    }

    .empty {
        padding: var(--space-5);
        color: var(--text-muted);
        text-align: center;
    }

    ul {
        margin: 0;
        padding: 0;
        list-style: none;
    }

    .row {
        position: relative;
        display: flex;
        align-items: center;
        border: 1px solid transparent;
        border-bottom-color: var(--row-separator);
        background: var(--surface);
        user-select: none;
    }

    .row:hover {
        border-color: var(--row-hover-border);
        background: var(--row-hover-face);
    }

    .row.ticked {
        border-color: var(--row-hover-border);
        background: var(--row-hover-face);
    }

    .row.selected {
        border-color: var(--row-selected-border);
        background: var(--row-selected-face);
    }

    /* Where dragged rows were: an empty slot of the same size. */
    .row.lifted {
        border: 1px dashed var(--row-selected-border);
        background: var(--surface-sunken);
    }

    .row.lifted > * {
        visibility: hidden;
    }

    .lead {
        flex: none;
        display: flex;
        align-items: center;
        padding-left: var(--space-1);
    }

    .star {
        display: grid;
        place-items: center;
        width: 24px;
        height: 24px;
        padding: 0;
        border: 1px solid transparent;
        border-radius: var(--radius-control);
        background: none;
        color: var(--text-muted);
        cursor: default;
    }

    .star:hover {
        border-color: var(--control-border-hover);
        background: var(--control-face-hover);
    }

    .open {
        flex: 1;
        display: flex;
        align-items: center;
        gap: var(--space-2);
        min-width: 0;
        padding: var(--space-2) var(--space-3) var(--space-2) var(--space-1);
        border: 0;
        background: none;
        color: inherit;
        font: inherit;
        text-align: left;
        cursor: default;
    }

    .open:focus-visible {
        outline: none;
        box-shadow: inset var(--focus-glow);
    }

    /* Always there, so read and unread rows line up. */
    .mark {
        flex: none;
        display: grid;
        place-items: center;
        width: 10px;
    }

    .dot {
        width: 8px;
        height: 8px;
        border: 1px solid var(--badge-border);
        border-radius: 50%;
        background: var(--badge-face);
    }

    .picture {
        position: relative;
        flex: none;
        display: grid;
    }

    .pile::before {
        content: '';
        position: absolute;
        inset: calc(-1 * var(--space-1)) calc(-1 * var(--space-1)) var(--space-1) var(--space-1);
        border: 1px solid var(--avatar-border);
        border-radius: 50%;
        background: var(--avatar-shine), var(--pile);
        box-shadow: var(--avatar-glass);
    }

    .count {
        flex: none;
        padding: 0 var(--space-1);
        border-radius: var(--radius-control);
        background: var(--badge-quiet-face);
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    .unread .count {
        color: var(--text);
        font-weight: 600;
    }

    .hidden {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
    }

    .lines {
        flex: 1;
        display: grid;
        gap: 2px;
        min-width: 0;
    }

    .top {
        display: flex;
        align-items: baseline;
        gap: var(--space-2);
        min-width: 0;
    }

    .sender {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .unread .sender,
    .unread .subject {
        font-weight: 600;
    }

    .date {
        flex: none;
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    .unread .date {
        color: var(--unread-mark);
        font-weight: 600;
    }

    .subject {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: var(--text-muted);
    }

    .unread .subject {
        color: var(--text);
    }

    /* A row's own tools: a small toolbar over its end, where the pointer is. */
    .tools {
        position: absolute;
        top: 50%;
        right: var(--space-2);
        padding: 1px;
        border: 1px solid var(--toolbar-border);
        border-radius: var(--radius-control);
        background: var(--toolbar-face);
        box-shadow: var(--menu-shadow);
        opacity: 0;
        pointer-events: none;
        transform: translateY(-50%);
        transition: opacity var(--motion-fast);
    }

    .row:hover .tools {
        opacity: 1;
        pointer-events: auto;
    }

    .dragging .row:hover .tools {
        opacity: 0;
        pointer-events: none;
    }

    .more {
        display: flex;
        justify-content: center;
        padding: var(--space-3);
    }

    /* Above everything and never in the way: what is under the pointer is where it drops. */
    .ghost {
        position: fixed;
        inset: auto;
        margin: 0;
        padding: 0;
        border: 0;
        background: none;
        overflow: visible;
        opacity: var(--drag-opacity);
        pointer-events: none;
        transition: transform var(--motion-soft), opacity var(--motion-soft);
    }

    /* Over a folder or tab that takes it: smaller, saying "let go here". */
    .ghost.over {
        transform: scale(var(--drag-shrink));
    }

    /* Let go: it falls into the middle of the target and fades there. */
    .ghost.landing {
        opacity: 0;
        transform: translate(var(--fall-x), var(--fall-y)) scale(var(--drag-land));
    }

    .ghost .row {
        border-color: var(--row-selected-border);
        border-radius: var(--radius-control);
        box-shadow: var(--menu-shadow);
    }

    /* More than one row: the edge of another behind the grabbed one. */
    .ghost.stack::before {
        content: '';
        position: absolute;
        inset: 4px -4px -4px 4px;
        border: 1px solid var(--row-selected-border);
        border-radius: var(--radius-control);
        background: var(--surface);
        box-shadow: var(--menu-shadow);
    }

    .ghost .row {
        position: relative;
    }

    .carried {
        position: absolute;
        top: calc(-1 * var(--space-2));
        right: calc(-1 * var(--space-2));
        padding: 0 var(--space-2);
        border: 1px solid var(--badge-border);
        border-radius: var(--radius-panel);
        background: var(--badge-face);
        color: var(--badge-text);
        font-size: var(--font-size-small);
        font-weight: 600;
    }
</style>

<script lang="ts">
    import type { Category, Conversation, Folder } from '$lib/api'
    import Avatar from '$lib/design/Avatar.svelte'
    import { avatarTone } from '$lib/design/avatar'
    import Button from '$lib/design/Button.svelte'
    import Icon from '$lib/design/Icon.svelte'
    import { t } from '$lib/i18n'
    import { categoryIcon, folderIcon, folderLabel, participantsLabel, senderLabel, senderPicture, shortDate, subjectLabel, type Pictures } from './workspace'

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
        // The account's address, named "me" among the people of a conversation.
        ownEmail: string
        pictures: Pictures
        // True when the server has older messages than the ones shown.
        hasMore: boolean
        busy: boolean
        onselect: (conversation: Conversation) => void
        oncategory: (category: Category) => void
        onflag: (conversation: Conversation) => void
        onmore: () => void
        onrefresh: () => void
    }

    let { folder, conversations, tabs, category, selectedId, ownEmail, pictures, hasMore, busy, onselect, oncategory, onflag, onmore, onrefresh }: Props = $props()

    const id = $props.id()
    const tabButtons: Partial<Record<Category, HTMLButtonElement>> = {}

    // Left and right walk the tabs, Home and End jump to the ends, as in any
    // row of tabs; a tab opens as soon as it is reached.
    const walk = (event: KeyboardEvent, index: number) => {
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

    // The picture behind a conversation's picture has the colour of someone
    // else who wrote in it, so the pile reads as more than one message.
    const pileTone = (conversation: Conversation): string => {
        const latest = conversation.latest.from[0]?.email.toLowerCase() ?? ''
        const other = conversation.senders.find((sender) => sender.email.toLowerCase() !== latest)

        return `var(--avatar-${avatarTone(other?.email ?? latest)})`
    }

    const isSelected = (conversation: Conversation): boolean => {
        return selectedId !== null && conversation.messageIds.includes(selectedId)
    }

    const now = new Date()

</script>

<section class="list">
    <header>
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
    </header>

    {#if tabs.length > 0}
        <div class="tabs" role="tablist" aria-label={t('list.categories')}>
            {#each tabs as tab, index (tab.category)}
                {@const name = t(`category.${tab.category}`)}
                <button
                    bind:this={tabButtons[tab.category]}
                    class="tab"
                    class:selected={tab.category === category}
                    type="button"
                    role="tab"
                    id={`${id}-tab-${tab.category}`}
                    aria-selected={tab.category === category}
                    aria-controls={`${id}-rows`}
                    aria-label={tab.unread > 0 ? t('list.categoryUnread', { name, count: String(tab.unread) }) : name}
                    title={t(`categoryHint.${tab.category}`)}
                    tabindex={tab.category === category ? 0 : -1}
                    onclick={() => oncategory(tab.category)}
                    onkeydown={(event) => walk(event, index)}
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
            <ul aria-label={t('list.messages')}>
                {#each conversations as conversation (conversation.latest.id)}
                    {@const message = conversation.latest}
                    {@const grouped = conversation.count > 1}
                    <li
                        class="row"
                        class:selected={isSelected(conversation)}
                        class:unread={conversation.unread > 0}
                        class:flagged={conversation.flagged}
                    >
                        <button
                            class="open"
                            type="button"
                            aria-current={isSelected(conversation)}
                            onclick={() => onselect(conversation)}
                        >
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
                        </button>
                        <button
                            class="flag"
                            type="button"
                            title={conversation.flagged ? t('toolbar.unflag') : t('toolbar.flag')}
                            aria-label={conversation.flagged ? t('toolbar.unflag') : t('toolbar.flag')}
                            aria-pressed={conversation.flagged}
                            onclick={() => onflag(conversation)}
                        >
                            <Icon name={conversation.flagged ? 'flag' : 'flagOff'} size={16} />
                        </button>
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

<style>
    .list {
        display: flex;
        flex-direction: column;
        min-width: 0;
        min-height: 0;
        background: var(--surface);
        color: var(--text);
    }

    header {
        flex: none;
        display: flex;
        align-items: center;
        gap: var(--space-2);
        min-height: 40px;
        padding: var(--space-1) var(--space-1) var(--space-1) var(--space-3);
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

    .unread-count {
        flex: none;
        font-size: var(--font-size-small);
        color: var(--text-muted);
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
        border: 1px solid transparent;
        border-bottom-color: var(--row-separator);
    }

    .row:hover {
        border-color: var(--row-hover-border);
        background: var(--row-hover-face);
    }

    .row.selected {
        border-color: var(--row-selected-border);
        background: var(--row-selected-face);
    }

    .open {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        width: 100%;
        padding: var(--space-2) var(--space-3) var(--space-2) var(--space-1);
        border: 0;
        background: none;
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
        /* Room for the flag, which sits at the end of this line. */
        padding-right: var(--space-5);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: var(--text-muted);
    }

    .unread .subject {
        color: var(--text);
    }

    .flag {
        position: absolute;
        right: var(--space-2);
        bottom: var(--space-1);
        display: grid;
        place-items: center;
        width: 24px;
        height: 24px;
        padding: 0;
        border: 1px solid transparent;
        border-radius: var(--radius-control);
        background: none;
        color: var(--text-muted);
        opacity: 0;
        cursor: default;
        transition: opacity var(--motion-fast);
    }

    /* A set flag is always shown; an unset one appears where the pointer or the keyboard is. */
    .flagged .flag,
    .row:hover .flag,
    .flag:focus-visible {
        opacity: 1;
    }

    .flag:hover,
    .flag:focus-visible {
        border-color: var(--control-border-hover);
        background: var(--control-face-hover);
        outline: none;
    }

    .more {
        display: flex;
        justify-content: center;
        padding: var(--space-3);
    }
</style>

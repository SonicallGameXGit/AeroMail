<script lang="ts">
    import type { MessageSummary } from '$lib/api'
    import Button from '$lib/design/Button.svelte'
    import { t } from '$lib/i18n'
    import { senderLabel, shortDate, subjectLabel } from './workspace'

    type Props = {
        messages: MessageSummary[]
        selectedId: number | null
        // True when the server has older messages than the ones shown.
        hasMore: boolean
        busy: boolean
        onselect: (message: MessageSummary) => void
        onmore: () => void
    }

    let { messages, selectedId, hasMore, busy, onselect, onmore }: Props = $props()

    const now = new Date()
</script>

<div class="list">
    {#if messages.length === 0}
        <p class="empty">{busy ? t('app.loading') : t('list.empty')}</p>
    {:else}
        <ul aria-label={t('list.messages')}>
            {#each messages as message (message.id)}
                <li>
                    <button
                        class="row"
                        class:selected={message.id === selectedId}
                        class:unread={!message.flags.seen}
                        type="button"
                        aria-current={message.id === selectedId}
                        onclick={() => onselect(message)}
                    >
                        <span class="top">
                            {#if !message.flags.seen}
                                <span class="dot" role="img" aria-label={t('list.unread')}></span>
                            {/if}
                            <span class="sender">{senderLabel(message)}</span>
                            {#if message.flags.flagged}
                                <span class="flag" role="img" aria-label={t('list.flagged')}>⚑</span>
                            {/if}
                            <span class="date">{shortDate(message.date, now)}</span>
                        </span>
                        <span class="subject">{subjectLabel(message)}</span>
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

<style>
    .list {
        min-height: 0;
        overflow-y: auto;
        background: var(--surface);
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
        display: grid;
        gap: 2px;
        width: 100%;
        padding: var(--space-2) var(--space-3);
        border: 1px solid transparent;
        border-bottom-color: var(--row-separator);
        background: none;
        color: var(--text);
        text-align: left;
    }

    .row:hover {
        border-color: var(--row-hover-border);
        background: var(--row-hover-face);
    }

    .row.selected {
        border-color: var(--row-selected-border);
        background: var(--row-selected-face);
    }

    .row:focus-visible {
        outline: none;
        box-shadow: inset var(--focus-glow);
    }

    .top {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        min-width: 0;
    }

    .dot {
        flex: none;
        width: 8px;
        height: 8px;
        border-radius: 50%;
        background: var(--unread-mark);
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

    .flag {
        flex: none;
        color: var(--flag-mark);
    }

    .date {
        flex: none;
        font-size: var(--font-size-small);
        color: var(--text-muted);
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

    .more {
        display: flex;
        justify-content: center;
        padding: var(--space-3);
    }
</style>

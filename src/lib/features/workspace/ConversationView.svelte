<script lang="ts">
    import { onMount, tick, type Snippet } from 'svelte'
    import { toMailError } from '$lib/api'
    import type { Api, Attachment, MailError, MessageContent, MessageSummary, ReplyMode } from '$lib/api'
    import Button from '$lib/design/Button.svelte'
    import Notice from '$lib/design/Notice.svelte'
    import { t } from '$lib/i18n'
    import ChatMessage from './ChatMessage.svelte'
    import {
        answerTarget,
        contactLabel,
        dayLabel,
        errorMessage,
        isOwnMessage,
        senderLabel,
        senderPicture,
        subjectLabel,
        type Pictures,
    } from './workspace'

    type Props = {
        api: Api
        // Oldest first.
        thread: MessageSummary[]
        // The account's address: its own messages are drawn on the other side.
        ownEmail: string
        pictures: Pictures
        // Remote images load in every message (the settings say so).
        remoteImages: boolean
        // The actions for the whole conversation, shown above it.
        toolbar?: Snippet
        onreply: (message: MessageSummary, mode: ReplyMode) => void
        // Sends `text` as a reply to everyone in the conversation; rejects when it could not be sent.
        onanswer: (text: string) => Promise<void>
        // Goes on writing `text` in the full editor.
        onexpand: (text: string) => void
        onsave: (message: MessageSummary, attachment: Attachment) => void
        onlink: (url: string) => void
    }

    let { api, thread, ownEmail, pictures, remoteImages, toolbar, onreply, onanswer, onexpand, onsave, onlink }: Props = $props()

    let contents = $state<Record<number, MessageContent>>({})
    let failures = $state<Record<number, MailError>>({})
    // Messages whose remote images the user asked for.
    let remote = $state<Record<number, true>>({})
    // Asked for already; not state, asking must not make anything ask again.
    const requested: Record<number, true> = {}

    let answer = $state('')
    let sending = $state(false)
    let answerError = $state<MailError | null>(null)

    let timeline = $state<HTMLElement | null>(null)
    // Whether the newest message is in view, so one that arrives is shown too.
    let atEnd = true

    const now = new Date()
    let target = $derived(answerTarget(thread, ownEmail))
    // The subject the conversation started with, before the "Re:"s.
    let title = $derived(thread[0] === undefined ? '' : subjectLabel(thread[0]))
    let participants = $derived(
        thread
            .flatMap((message) => message.from.slice(0, 1))
            .filter((sender, index, all) => all.findIndex((other) => other.email.toLowerCase() === sender.email.toLowerCase()) === index)
            .map((sender) => sender.email.toLowerCase() === ownEmail.toLowerCase() ? t('list.me') : contactLabel(sender))
            .join(', '),
    )

    const load = async (id: number, allowRemote: boolean) => {
        requested[id] = true
        delete failures[id]
        try {
            contents[id] = await api.openMessage(id, allowRemote)
        } catch (caught) {
            failures[id] = toMailError(caught)
            delete requested[id]
        }
    }

    // A message is downloaded once it comes near the visible part, so a long
    // conversation costs only what is read of it.
    const whenNear = (node: HTMLElement, id: number) => {
        const observer = new IntersectionObserver((entries) => {
            if (entries.some((entry) => entry.isIntersecting) && requested[id] !== true) {
                observer.disconnect()
                load(id, false)
            }
        }, { root: node.closest('.timeline'), rootMargin: '600px 0px' })
        observer.observe(node)

        return {
            destroy: () => observer.disconnect(),
        }
    }

    const showRemote = async (id: number) => {
        remote[id] = true
        await load(id, true)
    }

    const sameDay = (a: MessageSummary, b: MessageSummary): boolean => {
        return new Date(a.date * 1000).toDateString() === new Date(b.date * 1000).toDateString()
    }

    // A new day, or another sender than the message before: who wrote is shown again.
    const startsRun = (index: number): boolean => {
        const message = thread[index]
        const before = thread[index - 1]
        if (message === undefined || before === undefined) {
            return true
        }

        return !sameDay(before, message) || before.from[0]?.email.toLowerCase() !== message.from[0]?.email.toLowerCase()
    }

    const send = async () => {
        if (answer.trim() === '' || sending) {
            return
        }
        sending = true
        answerError = null
        try {
            await onanswer(answer)
            answer = ''
        } catch (caught) {
            answerError = toMailError(caught)
        } finally {
            sending = false
        }
    }

    const submit = (event: SubmitEvent) => {
        event.preventDefault()
        send()
    }

    // Enter starts a new line, as in any letter; Ctrl+Enter sends.
    const keys = (event: KeyboardEvent) => {
        if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
            event.preventDefault()
            send()
        }
    }

    // The field grows with what is written, up to a few lines, then scrolls.
    const grow = (field: HTMLTextAreaElement) => {
        field.style.height = 'auto'
        field.style.height = field.scrollHeight + 'px'
    }

    const scrolled = () => {
        if (timeline !== null) {
            atEnd = timeline.scrollHeight - timeline.scrollTop - timeline.clientHeight < timeline.clientHeight / 4
        }
    }

    const scrollTo = (id: number) => {
        timeline?.querySelector(`[data-message="${id}"]`)?.scrollIntoView({ block: 'start' })
    }

    // Opened at the first message not read yet, or at the newest one. What
    // loads above it later does not move it: the browser keeps it in place.
    onMount(() => {
        const unread = thread.find((message) => !message.flags.seen)
        const first = unread ?? thread.at(-1)
        if (first !== undefined) {
            scrollTo(first.id)
        }
    })

    // A message that arrives while the newest one is in view is shown too.
    let shownCount: number | null = null
    $effect(() => {
        const count = thread.length
        const before = shownCount
        shownCount = count
        const newest = thread.at(-1)
        if (before !== null && count > before && atEnd && newest !== undefined) {
            tick().then(() => scrollTo(newest.id))
        }
    })
</script>

<article class="conversation">
    {#if toolbar !== undefined}
        {@render toolbar()}
    {/if}

    <header>
        <h1>{title}</h1>
        <p>
            <span class="count">{t('conversation.count', { count: String(thread.length) })}</span>
            <span class="people">{participants}</span>
        </p>
    </header>

    <div class="timeline" bind:this={timeline} onscroll={scrolled} role="log" aria-label={t('conversation.label')}>
        {#each thread as message, index (message.id)}
            {#if index === 0 || !sameDay(thread[index - 1] ?? message, message)}
                <p class="day"><span>{dayLabel(message.date, now)}</span></p>
            {/if}
            <div class="slot" data-message={message.id} use:whenNear={message.id}>
                <ChatMessage
                    summary={message}
                    content={contents[message.id] ?? null}
                    failure={failures[message.id] ?? null}
                    mine={isOwnMessage(message, ownEmail)}
                    first={startsRun(index)}
                    picture={senderPicture(pictures, message)}
                    remoteAllowed={remoteImages || remote[message.id] === true}
                    onshowremote={() => showRemote(message.id)}
                    onsave={(attachment) => onsave(message, attachment)}
                    {onlink}
                    onreply={(mode) => onreply(message, mode)}
                    onretry={() => load(message.id, remote[message.id] === true)}
                />
            </div>
        {/each}
    </div>

    <form class="answer" novalidate onsubmit={submit}>
        {#if answerError !== null}
            <Notice kind="danger">
                {errorMessage(answerError)}
                <small>{t('error.details')}: {answerError.detail}</small>
            </Notice>
        {/if}
        <div class="row">
            <textarea
                rows="1"
                placeholder={t('conversation.replyTo', { name: target === null ? '' : senderLabel(target) })}
                aria-label={t('conversation.reply')}
                title={t('conversation.replyHint')}
                disabled={sending}
                bind:value={answer}
                oninput={(event) => grow(event.currentTarget)}
                onkeydown={keys}
            ></textarea>
            <Button variant="tool" icon="compose" label={t('conversation.expand')} onclick={() => onexpand(answer)} />
            <Button variant="accent" type="submit" icon="sent" disabled={sending || answer.trim() === ''}>
                {sending ? t('conversation.sending') : t('conversation.send')}
            </Button>
        </div>
    </form>
</article>

<style>
    .conversation {
        display: flex;
        flex-direction: column;
        min-width: 0;
        min-height: 0;
        background: var(--surface);
        color: var(--text);
    }

    header {
        flex: none;
        display: grid;
        gap: var(--space-1);
        padding: var(--space-3) var(--space-4);
        border-bottom: 1px solid var(--row-separator);
        background: var(--surface-sunken);
        user-select: text;
        -webkit-user-select: text;
    }

    h1 {
        font-size: var(--font-size-title);
        font-weight: 400;
        line-height: 1.25;
        color: var(--text-link);
        overflow-wrap: anywhere;
    }

    header p {
        display: flex;
        gap: var(--space-2);
        min-width: 0;
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    .count {
        flex: none;
    }

    .people {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .timeline {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
        min-height: 0;
        padding: var(--space-3) var(--space-4) var(--space-4);
        overflow-y: auto;
        background: var(--surface-sunken);
    }

    .slot {
        flex: none;
    }

    .day {
        flex: none;
        display: flex;
        align-items: center;
        gap: var(--space-3);
        margin: var(--space-2) 0;
        font-size: var(--font-size-small);
        color: var(--conversation-day-text);
    }

    .day::before,
    .day::after {
        content: '';
        flex: 1;
        border-top: 1px solid var(--row-separator);
    }

    .answer {
        flex: none;
        display: grid;
        gap: var(--space-2);
        padding: var(--space-2);
        border-top: 1px solid var(--toolbar-border);
        background: var(--toolbar-face);
    }

    .row {
        display: flex;
        align-items: flex-end;
        gap: var(--space-2);
    }

    textarea {
        flex: 1;
        min-width: 0;
        max-height: 30vh;
        padding: var(--space-1) var(--space-2);
        border: 1px solid var(--field-border);
        border-top-color: var(--field-border-top);
        border-radius: var(--radius-control);
        background: var(--field-background);
        box-shadow: var(--field-inner-shadow);
        resize: none;
    }

    textarea:focus-visible {
        outline: none;
        box-shadow: var(--field-inner-shadow), var(--focus-glow);
    }

    small {
        display: block;
        margin-top: var(--space-1);
        color: var(--text-muted);
        overflow-wrap: anywhere;
    }
</style>

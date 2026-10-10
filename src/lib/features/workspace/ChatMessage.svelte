<script lang="ts">
    import type { Attachment, MailError, MessageContent, MessageSummary, ReplyMode } from '$lib/api'
    import Avatar from '$lib/design/Avatar.svelte'
    import Button from '$lib/design/Button.svelte'
    import Icon from '$lib/design/Icon.svelte'
    import { t } from '$lib/i18n'
    import { catchLinks, fitHeight } from './frame'
    import { contactLabel, errorMessage, frameDocument, fullDate, quoteSelector, senderLabel, sizeLabel, timeLabel } from './workspace'

    type Props = {
        summary: MessageSummary
        // Null until it was downloaded.
        content: MessageContent | null
        failure: MailError | null
        // Written by the account itself: shown on the other side, without a picture.
        mine: boolean
        // The first of several messages in a row by the same sender: only it shows who wrote.
        first: boolean
        picture: string | null
        // True when remote images may load: asked for this message, or allowed in the settings.
        remoteAllowed: boolean
        onshowremote: () => void
        onsave: (attachment: Attachment) => void
        onlink: (url: string) => void
        onreply: (mode: ReplyMode) => void
        onretry: () => void
    }

    let { summary, content, failure, mine, first, picture, remoteAllowed, onshowremote, onsave, onlink, onreply, onretry }: Props = $props()

    // What the message repeats of earlier ones stays hidden until asked for:
    // those messages are right above it.
    let quoteShown = $state(false)
    // Whether the message's HTML holds a quote to hide; known once it is shown.
    let htmlQuote = $state(false)
    let text = $derived.by(() => {
        if (content === null || content.text === null) {
            return null
        }

        return quoteShown ? content.text : content.textWithoutQuote ?? content.text
    })
    let hasQuote = $derived(content !== null && (content.html === null ? content.textWithoutQuote !== null : htmlQuote))
    let name = $derived(mine ? t('conversation.you') : senderLabel(summary))
    let recipients = $derived(summary.to.concat(summary.cc).map(contactLabel).join(', '))

    let stopFitting: () => void = () => undefined

    const opened = (event: Event) => {
        const frame = event.currentTarget
        if (!(frame instanceof HTMLIFrameElement)) {
            return
        }

        catchLinks(frame, onlink)
        stopFitting()
        stopFitting = fitHeight(frame)
        if (!quoteShown) {
            htmlQuote = frame.contentDocument?.querySelector(quoteSelector) !== null
        }
    }

    $effect(() => () => stopFitting())
</script>

<article class="message" class:mine aria-label={`${name}, ${fullDate(summary.date)}`}>
    {#if !mine}
        <!-- Kept, unseen, under the first one of a row, so the bubbles line up. -->
        <span class="side" class:unseen={!first}>
            <Avatar name={senderLabel(summary)} seed={summary.from[0]?.email ?? ''} image={picture} />
        </span>
    {/if}

    <div class="bubble" class:wide={content !== null && content.html !== null}>
        <header>
            <span class="who">
                <strong>{name}</strong>
                {#if recipients !== ''}
                    <span class="to">{t('conversation.to', { names: recipients })}</span>
                {/if}
            </span>
            <time datetime={new Date(summary.date * 1000).toISOString()} title={fullDate(summary.date)}>{timeLabel(summary.date)}</time>
            <span class="tools">
                <Button variant="tool" size="small" icon="reply" label={t('toolbar.reply')} onclick={() => onreply('Reply')} />
                <Button variant="tool" size="small" icon="forward" label={t('toolbar.forward')} onclick={() => onreply('Forward')} />
            </span>
        </header>

        {#if failure !== null}
            <p class="failed" role="alert">
                <Icon name="warning" size={16} />
                <span>{t('conversation.loadFailed')} {errorMessage(failure)}</span>
                <Button variant="link" onclick={onretry}>{t('conversation.retry')}</Button>
            </p>
        {:else if content === null}
            <p class="loading">{t('app.loading')}</p>
        {:else}
            {#if content.remoteContentBlocked && !remoteAllowed}
                <p class="remote">
                    <Icon name="image" size={16} />
                    <span>{t('reader.remoteBlocked')}</span>
                    <Button variant="link" onclick={onshowremote}>{t('reader.showRemote')}</Button>
                </p>
            {/if}

            {#if content.html !== null}
                <iframe
                    title={t('reader.content')}
                    sandbox="allow-same-origin"
                    srcdoc={frameDocument(content.html, remoteAllowed, !quoteShown)}
                    onload={opened}
                ></iframe>
            {:else if text !== null && text.trim() !== ''}
                <div class="text">{text}</div>
            {:else}
                <p class="loading">{t('reader.noContent')}</p>
            {/if}

            {#if hasQuote}
                <button class="quote" type="button" aria-expanded={quoteShown} onclick={() => quoteShown = !quoteShown}>
                    <span aria-hidden="true">···</span>
                    {quoteShown ? t('conversation.hideQuote') : t('conversation.showQuote')}
                </button>
            {/if}

            {#if content.attachments.length > 0}
                <ul class="attachments" aria-label={t('reader.attachments')}>
                    {#each content.attachments as attachment (attachment.index)}
                        <li title={t('reader.save', { name: attachment.name })}>
                            <Button icon="attachment" onclick={() => onsave(attachment)}>
                                {attachment.name} · {sizeLabel(attachment.size)}
                            </Button>
                        </li>
                    {/each}
                </ul>
            {/if}
        {/if}
    </div>
</article>

<style>
    .message {
        display: flex;
        align-items: flex-end;
        gap: var(--space-2);
    }

    .mine {
        flex-direction: row-reverse;
    }

    .side {
        flex: none;
        display: grid;
    }

    .unseen {
        visibility: hidden;
    }

    .bubble {
        display: grid;
        gap: var(--space-2);
        min-width: 0;
        max-width: var(--bubble-width);
        padding: var(--space-2) var(--space-3);
        border: 1px solid var(--bubble-border);
        border-radius: var(--radius-panel);
        background: var(--bubble-face);
        box-shadow: var(--bubble-shadow);
        user-select: text;
        -webkit-user-select: text;
    }

    .mine .bubble {
        border-color: var(--bubble-own-border);
        background: var(--bubble-own-face);
    }

    /* Mail laid out as a page needs the width it was made for. */
    .bubble.wide {
        flex: 1;
        max-width: none;
    }

    header {
        display: flex;
        align-items: baseline;
        gap: var(--space-2);
        min-width: 0;
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    .who {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    strong {
        color: var(--text);
        font-weight: 600;
    }

    .to {
        margin-left: var(--space-1);
    }

    time {
        flex: none;
    }

    /* The answers to one message appear where the pointer or the keyboard is. */
    .tools {
        flex: none;
        display: flex;
        align-self: center;
        gap: 2px;
        opacity: 0;
        transition: opacity var(--motion-fast);
    }

    .message:hover .tools,
    .tools:focus-within {
        opacity: 1;
    }

    .text {
        white-space: pre-wrap;
        overflow-wrap: anywhere;
    }

    iframe {
        display: block;
        width: 100%;
        height: 0;
        border: 0;
        border-radius: var(--radius-control);
        /* Mail is written for a white page, whatever the app theme is; see Reader. */
        background: #ffffff;
        color-scheme: light;
    }

    .loading {
        color: var(--text-muted);
    }

    .failed,
    .remote {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: var(--space-2);
        font-size: var(--font-size-small);
    }

    .failed {
        color: var(--danger-text);
    }

    .remote {
        padding: var(--space-1) var(--space-2);
        border: 1px solid var(--warning-border);
        border-radius: var(--radius-control);
        background: var(--warning-background);
    }

    .quote {
        justify-self: start;
        display: inline-flex;
        align-items: center;
        gap: var(--space-2);
        padding: 0 var(--space-2);
        border: 1px solid var(--bubble-border);
        border-radius: var(--radius-control);
        background: var(--control-face);
        color: var(--text-muted);
        font-size: var(--font-size-small);
        cursor: default;
    }

    .quote:hover {
        border-color: var(--control-border-hover);
        background: var(--control-face-hover);
    }

    .quote:focus-visible {
        outline: none;
        box-shadow: var(--focus-glow);
    }

    .attachments {
        display: flex;
        flex-wrap: wrap;
        gap: var(--space-2);
        margin: 0;
        padding: 0;
        list-style: none;
    }
</style>

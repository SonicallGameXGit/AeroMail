<script lang="ts">
    import type { Snippet } from 'svelte'
    import type { Attachment, MessageContent } from '$lib/api'
    import Avatar from '$lib/design/Avatar.svelte'
    import Button from '$lib/design/Button.svelte'
    import Icon from '$lib/design/Icon.svelte'
    import Notice from '$lib/design/Notice.svelte'
    import { t } from '$lib/i18n'
    import { contactFull, contactLabel, frameDocument, fullDate, isSafeLink, sizeLabel, subjectLabel } from './workspace'

    type Props = {
        content: MessageContent | null
        busy: boolean
        // True when remote images may load: asked for this message, or allowed in the settings.
        remoteAllowed: boolean
        // The sender's own picture, when one was found.
        picture: string | null
        // The actions for the open message, shown above it.
        toolbar?: Snippet
        onshowremote: () => void
        onsave: (attachment: Attachment) => void
        onlink: (url: string) => void
    }

    let { content, busy, remoteAllowed, picture, toolbar, onshowremote, onsave, onlink }: Props = $props()

    // The frame cannot run scripts, so links are caught from the outside and
    // handed to the system browser instead of navigating inside the app.
    const catchLinks = (event: Event) => {
        const frame = event.currentTarget
        if (!(frame instanceof HTMLIFrameElement) || frame.contentDocument === null) {
            return
        }
        // Elements inside the frame belong to the frame's own window, so they
        // must be checked against its classes, not the app's.
        const view = frame.contentDocument.defaultView
        if (view === null) {
            return
        }

        frame.contentDocument.addEventListener('click', (click) => {
            if (!(click.target instanceof view.Element)) {
                return
            }
            const link = click.target.closest('a')
            if (link === null) {
                return
            }

            click.preventDefault()
            const url = link.getAttribute('href')
            if (url !== null && isSafeLink(url)) {
                onlink(url)
            }
        })
    }
</script>

<article class="reader">
    {#if toolbar !== undefined}
        {@render toolbar()}
    {/if}

    {#if content === null}
        <div class="empty">
            {#if !busy}
                <Icon name="mailOpen" size={72} />
            {/if}
            <p>{busy ? t('app.loading') : t('reader.empty')}</p>
        </div>
    {:else}
        {@const summary = content.summary}
        {@const sender = summary.from[0]}
        <header>
            <h1>{subjectLabel(summary)}</h1>
            <div class="meta">
                <Avatar
                    name={sender === undefined ? '' : contactLabel(sender)}
                    seed={sender?.email ?? ''}
                    image={picture}
                    size="large"
                />
                <dl>
                    <dt class="hidden">{t('reader.from')}</dt>
                    <dd class="from">
                        {#each summary.from as contact (contact.email)}
                            <span class="contact">
                                {#if contact.name !== null}
                                    <strong>{contact.name}</strong>
                                    <span class="address">&lt;{contact.email}&gt;</span>
                                {:else}
                                    <strong>{contact.email}</strong>
                                {/if}
                            </span>
                        {/each}
                    </dd>
                    <dt>{t('reader.to')}</dt>
                    <dd>{summary.to.map(contactFull).join(', ')}</dd>
                    {#if summary.cc.length > 0}
                        <dt>{t('reader.cc')}</dt>
                        <dd>{summary.cc.map(contactFull).join(', ')}</dd>
                    {/if}
                </dl>
                <time>{fullDate(summary.date)}</time>
            </div>
        </header>

        {#if content.remoteContentBlocked}
            <div class="strip">
                <Notice kind="warning">
                    <span class="notice-row">
                        <Icon name="image" size={20} />
                        <span class="grow">{t('reader.remoteBlocked')}</span>
                        <Button onclick={onshowremote}>{t('reader.showRemote')}</Button>
                    </span>
                </Notice>
            </div>
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

        {#if content.html !== null}
            <iframe
                title={t('reader.content')}
                sandbox="allow-same-origin"
                srcdoc={frameDocument(content.html, remoteAllowed)}
                onload={catchLinks}
            ></iframe>
        {:else if content.text !== null && content.text.trim() !== ''}
            <pre>{content.text}</pre>
        {:else}
            <p class="empty">{t('reader.noContent')}</p>
        {/if}
    {/if}
</article>

<style>
    .reader {
        display: flex;
        flex-direction: column;
        min-width: 0;
        min-height: 0;
        background: var(--surface);
        color: var(--text);
    }

    .empty {
        display: grid;
        justify-items: center;
        gap: var(--space-3);
        margin: auto;
        padding: var(--space-5);
        color: var(--text-muted);
    }

    .empty :global(svg) {
        opacity: 0.6;
    }

    header {
        flex: none;
        display: grid;
        gap: var(--space-3);
        padding: var(--space-4);
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

    .meta {
        display: flex;
        align-items: flex-start;
        gap: var(--space-3);
    }

    dl {
        flex: 1;
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        gap: 0 var(--space-2);
        min-width: 0;
        margin: 0;
    }

    dt {
        color: var(--text-muted);
    }

    /* The sender needs no caption: it is the line next to the picture. */
    dt.hidden {
        position: absolute;
        width: 1px;
        height: 1px;
        overflow: hidden;
        clip-path: inset(50%);
    }

    dd {
        margin: 0;
        color: var(--text-muted);
        overflow-wrap: anywhere;
    }

    dd.from {
        grid-column: 1 / 3;
        display: flex;
        flex-wrap: wrap;
        gap: 0 var(--space-3);
        color: var(--text);
    }

    .address {
        color: var(--text-muted);
    }

    time {
        flex: none;
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    .strip {
        padding: var(--space-2) var(--space-4) 0;
    }

    .notice-row {
        display: flex;
        align-items: center;
        gap: var(--space-3);
    }

    .grow {
        flex: 1;
    }

    .attachments {
        display: flex;
        flex-wrap: wrap;
        gap: var(--space-2);
        margin: 0;
        padding: var(--space-2) var(--space-4);
        border-bottom: 1px solid var(--row-separator);
        list-style: none;
    }

    iframe {
        flex: 1;
        min-height: 0;
        width: 100%;
        border: 0;
        /* Mail is written for a white page, whatever the app theme is. */
        background: #ffffff;
    }

    pre {
        flex: 1;
        min-height: 0;
        margin: 0;
        padding: var(--space-4);
        overflow: auto;
        font: inherit;
        white-space: pre-wrap;
        overflow-wrap: anywhere;
        user-select: text;
        -webkit-user-select: text;
    }
</style>

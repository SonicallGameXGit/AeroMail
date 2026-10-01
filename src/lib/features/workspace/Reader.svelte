<script lang="ts">
    import type { Attachment, MessageContent } from '$lib/api'
    import Button from '$lib/design/Button.svelte'
    import Notice from '$lib/design/Notice.svelte'
    import { t } from '$lib/i18n'
    import { contactFull, frameDocument, fullDate, isSafeLink, sizeLabel, subjectLabel } from './workspace'

    type Props = {
        content: MessageContent | null
        busy: boolean
        // True once the user asked to see remote images of this message.
        remoteAllowed: boolean
        onshowremote: () => void
        onsave: (attachment: Attachment) => void
        onlink: (url: string) => void
    }

    let { content, busy, remoteAllowed, onshowremote, onsave, onlink }: Props = $props()

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
    {#if content === null}
        <p class="empty">{busy ? t('app.loading') : t('reader.empty')}</p>
    {:else}
        {@const summary = content.summary}
        <header>
            <h1>{subjectLabel(summary)}</h1>
            <dl>
                <dt>{t('reader.from')}</dt>
                <dd>{summary.from.map(contactFull).join(', ')}</dd>
                <dt>{t('reader.to')}</dt>
                <dd>{summary.to.map(contactFull).join(', ')}</dd>
                {#if summary.cc.length > 0}
                    <dt>{t('reader.cc')}</dt>
                    <dd>{summary.cc.map(contactFull).join(', ')}</dd>
                {/if}
            </dl>
            <time>{fullDate(summary.date)}</time>
        </header>

        {#if content.remoteContentBlocked}
            <div class="strip">
                <Notice kind="warning">
                    <span class="notice-row">
                        <span>{t('reader.remoteBlocked')}</span>
                        <Button onclick={onshowremote}>{t('reader.showRemote')}</Button>
                    </span>
                </Notice>
            </div>
        {/if}

        {#if content.attachments.length > 0}
            <ul class="attachments" aria-label={t('reader.attachments')}>
                {#each content.attachments as attachment (attachment.index)}
                    <li>
                        <Button onclick={() => onsave(attachment)}>
                            <span title={t('reader.save', { name: attachment.name })}>
                                {attachment.name} · {sizeLabel(attachment.size)}
                            </span>
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
        margin: auto;
        padding: var(--space-5);
        color: var(--text-muted);
    }

    header {
        padding: var(--space-4);
        border-bottom: 1px solid var(--row-separator);
        background: var(--surface-sunken);
        user-select: text;
        -webkit-user-select: text;
    }

    h1 {
        font-size: var(--font-size-title);
        font-weight: 400;
        color: var(--text-link);
        overflow-wrap: anywhere;
    }

    dl {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        gap: 2px var(--space-3);
        margin: var(--space-2) 0;
    }

    dt {
        color: var(--text-muted);
    }

    dd {
        margin: 0;
        overflow-wrap: anywhere;
    }

    time {
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    .strip {
        padding: var(--space-2) var(--space-4) 0;
    }

    .notice-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: var(--space-3);
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

<script lang="ts">
    import type { Folder, MessageSummary, ReplyMode } from '$lib/api'
    import Button from '$lib/design/Button.svelte'
    import Menu from '$lib/design/Menu.svelte'
    import MenuItem from '$lib/design/MenuItem.svelte'
    import { t } from '$lib/i18n'
    import { folderIcon, folderLabel } from './workspace'

    type Props = {
        // The message the actions apply to.
        message: MessageSummary
        // False when the account has no archive folder.
        canArchive: boolean
        moveTargets: Folder[]
        onreply: (mode: ReplyMode) => void
        onarchive: () => void
        onmove: (folderId: number) => void
        ondelete: () => void
        onseen: () => void
        onflag: () => void
    }

    let { message, canArchive, moveTargets, onreply, onarchive, onmove, ondelete, onseen, onflag }: Props = $props()
</script>

<!-- Three groups, by what they do: answer the message, put it away, mark it. -->
<div class="actions" role="toolbar" aria-label={t('toolbar.label')}>
    <div class="group answer" role="group">
        <Button icon="reply" onclick={() => onreply('Reply')}>{t('toolbar.reply')}</Button>
        <Button icon="replyAll" label={t('toolbar.replyAll')} onclick={() => onreply('ReplyAll')} />
        <Button icon="forward" label={t('toolbar.forward')} onclick={() => onreply('Forward')} />
    </div>

    <span class="divider"></span>

    <div class="group" role="group">
        <Button variant="tool" icon="archive" label={t('toolbar.archive')} disabled={!canArchive} onclick={onarchive} />
        <Menu label={t('toolbar.moveTo')} icon="move" disabled={moveTargets.length === 0}>
            {#snippet children(close)}
                {#each moveTargets as folder (folder.id)}
                    <MenuItem
                        icon={folderIcon(folder)}
                        onclick={() => {
                            close()
                            onmove(folder.id)
                        }}
                    >{folderLabel(folder)}</MenuItem>
                {/each}
            {/snippet}
        </Menu>
        <Button variant="tool" tone="danger" icon="trash" label={t('toolbar.delete')} onclick={ondelete} />
    </div>

    <span class="spacer"></span>

    <div class="group" role="group">
        <Button
            variant="tool"
            icon={message.flags.seen ? 'mailUnread' : 'mailOpen'}
            label={message.flags.seen ? t('toolbar.markUnread') : t('toolbar.markRead')}
            onclick={onseen}
        />
        <Button
            variant="tool"
            icon={message.flags.flagged ? 'flag' : 'flagOff'}
            label={message.flags.flagged ? t('toolbar.unflag') : t('toolbar.flag')}
            onclick={onflag}
        />
    </div>
</div>

<style>
    .actions {
        flex: none;
        display: flex;
        align-items: center;
        gap: var(--space-2);
        min-height: 40px;
        padding: var(--space-1) var(--space-2);
        border-bottom: 1px solid var(--toolbar-border);
        background: var(--toolbar-face);
    }

    .group {
        display: flex;
        align-items: center;
        gap: 2px;
    }

    /* The three ways to answer are one control cut in three. */
    .answer {
        gap: 0;
    }

    .answer :global(button:not(:first-child)) {
        margin-left: -1px;
        border-top-left-radius: 0;
        border-bottom-left-radius: 0;
    }

    .answer :global(button:not(:last-child)) {
        border-top-right-radius: 0;
        border-bottom-right-radius: 0;
    }

    .answer :global(button:hover),
    .answer :global(button:focus-visible) {
        position: relative;
    }

    .divider {
        align-self: stretch;
        width: 1px;
        margin: var(--space-1) 0;
        background: var(--toolbar-border);
    }

    .spacer {
        flex: 1;
    }
</style>

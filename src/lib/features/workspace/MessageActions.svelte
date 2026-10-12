<script lang="ts">
    import type { Category, Folder, ReplyMode } from '$lib/api'
    import Button from '$lib/design/Button.svelte'
    import Menu from '$lib/design/Menu.svelte'
    import MenuItem from '$lib/design/MenuItem.svelte'
    import { t } from '$lib/i18n'
    import { categoryIcon, folderIcon, folderLabel } from './workspace'

    type Props = {
        // What the open message, or conversation as a whole, is: read when
        // nothing in it is unread, starred when anything in it is.
        seen: boolean
        starred: boolean
        // False when there is nowhere to archive to, or it is there already.
        canArchive: boolean
        // To Junk, back out of it, or not offered where there is no Junk.
        spam: 'report' | 'undo' | null
        moveTargets: Folder[]
        // The tabs of a sorted inbox the sender can be put in; none outside
        // the inbox, while it is not sorted, or without a sender.
        categories: Category[]
        // Where the open conversation is shown now.
        category: Category | null
        sender: string
        // Whether the user put the sender in a category before.
        sorted: boolean
        onreply: (mode: ReplyMode) => void
        onarchive: () => void
        onmove: (folderId: number) => void
        // A category for every message from the sender; null gives the
        // sender back to the automatic rules.
        oncategory: (category: Category | null) => void
        ondelete: () => void
        onspam: () => void
        onseen: () => void
        onstar: () => void
        // Given when the message covers the list: goes back to it.
        onback?: () => void
    }

    let { seen, starred, canArchive, spam, moveTargets, categories, category, sender, sorted, onreply, onarchive, onmove, oncategory, ondelete, onspam, onseen, onstar, onback }: Props = $props()
</script>

<!-- Three groups, by what they do: answer the message, put it away, mark it. -->
<div class="actions" role="toolbar" aria-label={t('toolbar.label')}>
    {#if onback !== undefined}
        <Button variant="tool" icon="back" label={t('toolbar.back')} onclick={onback} />
        <span class="divider"></span>
    {/if}
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
        {#if categories.length > 0}
            <Menu label={t('toolbar.category')} icon="tabs">
                {#snippet children(close)}
                    {#each categories as choice (choice)}
                        {@const name = t(`category.${choice}`)}
                        <MenuItem
                            icon={categoryIcon(choice)}
                            current={choice === category}
                            hint={t('toolbar.categoryHint', { sender, name })}
                            onclick={() => {
                                close()
                                oncategory(choice)
                            }}
                        >{name}</MenuItem>
                    {/each}
                    {#if sorted}
                        <MenuItem
                            hint={t('toolbar.categoryAutoHint', { sender })}
                            onclick={() => {
                                close()
                                oncategory(null)
                            }}
                        >{t('toolbar.categoryAuto')}</MenuItem>
                    {/if}
                {/snippet}
            </Menu>
        {/if}
        {#if spam !== null}
            <Button
                variant="tool"
                icon={spam === 'report' ? 'junk' : 'inbox'}
                label={spam === 'report' ? t('toolbar.spam') : t('toolbar.notSpam')}
                onclick={onspam}
            />
        {/if}
        <Button variant="tool" tone="danger" icon="trash" label={t('toolbar.delete')} onclick={ondelete} />
    </div>

    <span class="spacer"></span>

    <div class="group" role="group">
        <Button
            variant="tool"
            icon={seen ? 'mailUnread' : 'mailOpen'}
            label={seen ? t('toolbar.markUnread') : t('toolbar.markRead')}
            onclick={onseen}
        />
        <Button
            variant="tool"
            icon={starred ? 'star' : 'starOff'}
            label={starred ? t('toolbar.unstar') : t('toolbar.star')}
            onclick={onstar}
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

<script lang="ts">
    import type { Folder } from '$lib/api'
    import Button from '$lib/design/Button.svelte'
    import Menu from '$lib/design/Menu.svelte'
    import MenuItem from '$lib/design/MenuItem.svelte'
    import { t } from '$lib/i18n'
    import { folderIcon, folderLabel, type ListAction } from './workspace'

    type Props = {
        // Read when nothing in the rows is unread; starred when everything is.
        seen: boolean
        starred: boolean
        canArchive: boolean
        // To Junk, back out of it, or not offered where there is no Junk.
        spam: 'report' | 'undo' | null
        // Shown with the star and the folders to move to (the selection's
        // bar); a row's own tools leave them out, its star is beside it.
        full?: boolean
        moveTargets?: Folder[]
        // Reached by the pointer only: a row's tools have keys of their own.
        tabindex?: number
        // Names the group when it stands for more than one row.
        label?: string
        onact: (action: ListAction) => void
    }

    let { seen, starred, canArchive, spam, full = false, moveTargets = [], tabindex, label, onact }: Props = $props()
</script>

<span class="tools" role="group" aria-label={label}>
    <Button variant="tool" icon="archive" label={t('toolbar.archive')} {tabindex} disabled={!canArchive} onclick={() => onact({ kind: 'archive' })} />
    {#if spam !== null}
        <Button
            variant="tool"
            icon={spam === 'report' ? 'junk' : 'inbox'}
            label={spam === 'report' ? t('toolbar.spam') : t('toolbar.notSpam')}
            {tabindex}
            onclick={() => onact({ kind: 'spam' })}
        />
    {/if}
    <Button variant="tool" tone="danger" icon="trash" label={t('toolbar.delete')} {tabindex} onclick={() => onact({ kind: 'delete' })} />
    <Button
        variant="tool"
        icon={seen ? 'mailUnread' : 'mailOpen'}
        label={seen ? t('toolbar.markUnread') : t('toolbar.markRead')}
        {tabindex}
        onclick={() => onact({ kind: 'seen', seen: !seen })}
    />
    {#if full}
        <Button
            variant="tool"
            icon={starred ? 'star' : 'starOff'}
            label={starred ? t('toolbar.unstar') : t('toolbar.star')}
            onclick={() => onact({ kind: 'star', star: !starred })}
        />
        <Menu label={t('toolbar.moveTo')} icon="move" disabled={moveTargets.length === 0}>
            {#snippet children(close)}
                {#each moveTargets as folder (folder.id)}
                    <MenuItem
                        icon={folderIcon(folder)}
                        onclick={() => {
                            close()
                            onact({ kind: 'move', folderId: folder.id })
                        }}
                    >{folderLabel(folder)}</MenuItem>
                {/each}
            {/snippet}
        </Menu>
    {/if}
</span>

<style>
    .tools {
        display: flex;
        align-items: center;
        gap: 2px;
    }
</style>

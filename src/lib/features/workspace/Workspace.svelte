<script lang="ts">
    import { onMount } from 'svelte'
    import { toMailError } from '$lib/api'
    import type { Account, Api, Attachment, Folder, MailError, MessageContent, MessageSummary, ReplyMode } from '$lib/api'
    import Backdrop from '$lib/design/Backdrop.svelte'
    import Button from '$lib/design/Button.svelte'
    import GlassWindow from '$lib/design/GlassWindow.svelte'
    import Modal from '$lib/design/Modal.svelte'
    import Notice from '$lib/design/Notice.svelte'
    import AccountSetup from '$lib/features/accounts/AccountSetup.svelte'
    import { t } from '$lib/i18n'
    import Composer from './Composer.svelte'
    import MessageList from './MessageList.svelte'
    import Reader from './Reader.svelte'
    import Sidebar from './Sidebar.svelte'
    import { emptyForm, errorMessage, folderLabel, formFromDraft, toOutgoing, type ComposeForm } from './workspace'

    let { api }: { api: Api } = $props()

    // How often the open folder is checked for new mail.
    const refreshEveryMs = 2 * 60 * 1000

    // null until the account list was loaded.
    let accounts = $state<Account[] | null>(null)
    let folders = $state<Record<number, Folder[]>>({})
    let selectedFolder = $state<Folder | null>(null)
    let messages = $state<MessageSummary[]>([])
    let selectedId = $state<number | null>(null)
    let content = $state<MessageContent | null>(null)
    let remoteAllowed = $state(false)

    let syncing = $state(false)
    let opening = $state(false)
    let error = $state<MailError | null>(null)

    let addingAccount = $state(false)
    let form = $state<ComposeForm | null>(null)
    let sending = $state(false)
    let sendError = $state<MailError | null>(null)

    let selected = $derived(messages.find((message) => message.id === selectedId) ?? null)
    let accountFolders = $derived(selectedFolder === null ? [] : folders[selectedFolder.accountId] ?? [])
    let archiveFolder = $derived(accountFolders.find((folder) => folder.role === 'Archive' && folder.selectable) ?? null)
    let moveTargets = $derived(accountFolders.filter((folder) => folder.selectable && folder.id !== selectedFolder?.id))
    let hasMore = $derived(selectedFolder !== null && messages.length < selectedFolder.total)

    // Runs an action and shows its failure in the error bar instead of losing it.
    const guard = async (action: () => Promise<void>) => {
        try {
            await action()
        } catch (caught) {
            error = toMailError(caught)
        }
    }

    const reloadFolders = async (accountId: number) => {
        const list = await api.listFolders(accountId)
        folders = { ...folders, [accountId]: list }
        if (selectedFolder !== null && selectedFolder.accountId === accountId) {
            selectedFolder = list.find((folder) => folder.id === selectedFolder?.id) ?? selectedFolder
        }
    }

    const sync = async (folder: Folder, grow: boolean) => {
        syncing = true
        try {
            const fresh = await api.syncFolder(folder.id, grow)
            // The user may have moved on to another folder in the meantime.
            if (selectedFolder?.id === folder.id) {
                messages = fresh
                if (selectedId !== null && !fresh.some((message) => message.id === selectedId)) {
                    selectedId = null
                    content = null
                }
            }
            await reloadFolders(folder.accountId)
        } finally {
            syncing = false
        }
    }

    const selectFolder = (folder: Folder) => guard(async () => {
        selectedFolder = folder
        selectedId = null
        content = null
        // Cached messages show at once; the server is asked right after.
        messages = await api.listMessages(folder.id)
        await sync(folder, false)
    })

    const load = () => guard(async () => {
        const list = await api.listAccounts()
        for (const account of list) {
            folders = { ...folders, [account.id]: await api.listFolders(account.id) }
        }
        accounts = list

        const first = list[0]
        if (first === undefined) {
            return
        }
        const inbox = (folders[first.id] ?? []).find((folder) => folder.role === 'Inbox')
        if (inbox !== undefined) {
            await selectFolder(inbox)
        }
    })

    const refresh = () => guard(async () => {
        if (selectedFolder === null) {
            return
        }
        const folder = selectedFolder
        folders = { ...folders, [folder.accountId]: await api.refreshFolders(folder.accountId) }
        await sync(folder, false)
    })

    const patch = (id: number, change: (message: MessageSummary) => MessageSummary) => {
        messages = messages.map((message) => message.id === id ? change(message) : message)
        if (content !== null && content.summary.id === id) {
            content = { ...content, summary: change(content.summary) }
        }
    }

    const setSeen = async (message: MessageSummary, seen: boolean) => {
        await api.setSeen([message.id], seen)
        patch(message.id, (current) => ({ ...current, flags: { ...current.flags, seen } }))
        await reloadFolders(selectedFolder?.accountId ?? 0)
    }

    const openMessage = (message: MessageSummary) => guard(async () => {
        selectedId = message.id
        content = null
        remoteAllowed = false
        opening = true
        try {
            const opened = await api.openMessage(message.id, false)
            if (selectedId !== message.id) {
                return
            }
            content = opened
            if (!message.flags.seen) {
                await setSeen(message, true)
            }
        } finally {
            opening = false
        }
    })

    const showRemote = () => guard(async () => {
        if (selected === null) {
            return
        }
        const id = selected.id
        const opened = await api.openMessage(id, true)
        if (selectedId === id) {
            content = opened
            remoteAllowed = true
        }
    })

    const toggleSeen = () => guard(async () => {
        if (selected !== null) {
            await setSeen(selected, !selected.flags.seen)
        }
    })

    const toggleFlagged = () => guard(async () => {
        if (selected === null) {
            return
        }
        const flagged = !selected.flags.flagged
        await api.setFlagged([selected.id], flagged)
        patch(selected.id, (current) => ({ ...current, flags: { ...current.flags, flagged } }))
    })

    // After a message leaves the folder, the one below it takes its place.
    const leave = async (id: number) => {
        const index = messages.findIndex((message) => message.id === id)
        messages = messages.filter((message) => message.id !== id)
        selectedId = null
        content = null
        if (selectedFolder !== null) {
            await reloadFolders(selectedFolder.accountId)
        }

        const next = messages[index] ?? messages[index - 1]
        if (next !== undefined) {
            await openMessage(next)
        }
    }

    const remove = () => guard(async () => {
        if (selected === null) {
            return
        }
        const id = selected.id
        await api.deleteMessages([id])
        await leave(id)
    })

    const moveTo = (folderId: number) => guard(async () => {
        if (selected === null) {
            return
        }
        const id = selected.id
        await api.moveMessages([id], folderId)
        await leave(id)
    })

    const chooseTarget = (event: Event) => {
        if (!(event.currentTarget instanceof HTMLSelectElement)) {
            return
        }
        const folderId = Number(event.currentTarget.value)
        event.currentTarget.value = ''
        if (Number.isInteger(folderId) && folderId > 0) {
            moveTo(folderId)
        }
    }

    const compose = () => {
        const accountId = selectedFolder?.accountId ?? accounts?.[0]?.id
        if (accountId === undefined) {
            return
        }
        sendError = null
        form = emptyForm(accountId)
    }

    const reply = (mode: ReplyMode) => guard(async () => {
        if (selected === null || selectedFolder === null) {
            return
        }
        const draft = await api.replyDraft(selected.id, mode)
        sendError = null
        form = formFromDraft(selectedFolder.accountId, draft)
    })

    const send = async () => {
        if (form === null) {
            return
        }
        sending = true
        sendError = null
        try {
            await api.sendMessage(toOutgoing(form))
            form = null
            if (selectedFolder !== null && selectedFolder.role === 'Sent') {
                await sync(selectedFolder, false)
            }
        } catch (caught) {
            sendError = toMailError(caught)
        } finally {
            sending = false
        }
    }

    const saveAttachment = (attachment: Attachment) => guard(async () => {
        if (selected !== null) {
            await api.saveAttachment(selected.id, attachment)
        }
    })

    const openLink = (url: string) => guard(() => api.openExternal(url))

    const accountAdded = () => {
        addingAccount = false
        load()
    }

    const removeAccount = (account: Account) => guard(async () => {
        await api.removeAccount(account.id)
        if (selectedFolder !== null && selectedFolder.accountId === account.id) {
            selectedFolder = null
            messages = []
            selectedId = null
            content = null
        }
        await load()
    })

    onMount(() => {
        load()

        const timer = setInterval(() => {
            if (selectedFolder !== null && !syncing) {
                const folder = selectedFolder
                // A background check that fails stays quiet; the next one retries.
                sync(folder, false).catch(() => undefined)
            }
        }, refreshEveryMs)

        return () => clearInterval(timer)
    })
</script>

{#if accounts === null}
    <Backdrop>
        {#if error !== null}
            <GlassWindow title={t('app.name')}>
                <Notice kind="danger">
                    {errorMessage(error)}
                    <small>{t('error.details')}: {error.detail}</small>
                </Notice>
            </GlassWindow>
        {/if}
    </Backdrop>
{:else if accounts.length === 0}
    <Backdrop>
        <GlassWindow title={t('app.name')}>
            <AccountSetup {api} onadded={accountAdded} />
        </GlassWindow>
    </Backdrop>
{:else}
    <Backdrop fill>
        <div class="frame">
            <div class="toolbar" role="toolbar" aria-label={t('app.name')}>
                <Button variant="accent" onclick={compose}>{t('toolbar.compose')}</Button>
                <span class="gap"></span>
                <Button disabled={selected === null} onclick={() => reply('Reply')}>{t('toolbar.reply')}</Button>
                <Button disabled={selected === null} onclick={() => reply('ReplyAll')}>{t('toolbar.replyAll')}</Button>
                <Button disabled={selected === null} onclick={() => reply('Forward')}>{t('toolbar.forward')}</Button>
                <span class="gap"></span>
                <Button
                    disabled={selected === null || archiveFolder === null}
                    onclick={() => archiveFolder !== null && moveTo(archiveFolder.id)}
                >{t('toolbar.archive')}</Button>
                <Button disabled={selected === null} onclick={remove}>{t('toolbar.delete')}</Button>
                <select
                    class="move"
                    aria-label={t('toolbar.moveTo')}
                    disabled={selected === null || moveTargets.length === 0}
                    onchange={chooseTarget}
                >
                    <option value="">{t('toolbar.moveTo')}</option>
                    {#each moveTargets as folder (folder.id)}
                        <option value={folder.id}>{folderLabel(folder)}</option>
                    {/each}
                </select>
                <span class="gap"></span>
                <Button disabled={selected === null} onclick={toggleSeen}>
                    {selected !== null && !selected.flags.seen ? t('toolbar.markRead') : t('toolbar.markUnread')}
                </Button>
                <Button disabled={selected === null} onclick={toggleFlagged}>
                    {selected !== null && selected.flags.flagged ? t('toolbar.unflag') : t('toolbar.flag')}
                </Button>
                <span class="spacer"></span>
                <Button disabled={syncing || selectedFolder === null} onclick={refresh}>{t('toolbar.refresh')}</Button>
            </div>

            {#if error !== null}
                <div class="error">
                    <Notice kind="danger">
                        <span class="error-row">
                            <span>
                                {errorMessage(error)}
                                <small>{t('error.details')}: {error.detail}</small>
                            </span>
                            <Button onclick={() => error = null}>{t('app.dismiss')}</Button>
                        </span>
                    </Notice>
                </div>
            {/if}

            <div class="panes">
                <Sidebar
                    {accounts}
                    {folders}
                    selectedFolderId={selectedFolder?.id ?? null}
                    onselect={selectFolder}
                    onadd={() => addingAccount = true}
                    onremove={removeAccount}
                />
                <MessageList
                    {messages}
                    {selectedId}
                    {hasMore}
                    busy={syncing}
                    onselect={openMessage}
                    onmore={() => guard(async () => {
                        if (selectedFolder !== null) {
                            await sync(selectedFolder, true)
                        }
                    })}
                />
                <Reader
                    {content}
                    busy={opening}
                    {remoteAllowed}
                    onshowremote={showRemote}
                    onsave={saveAttachment}
                    onlink={openLink}
                />
            </div>
        </div>
    </Backdrop>

    {#if addingAccount}
        <Modal title={t('sidebar.addAccount')}>
            <AccountSetup {api} onadded={accountAdded} oncancel={() => addingAccount = false} />
        </Modal>
    {/if}

    {#if form !== null}
        <Modal title={t('compose.title')} wide>
            <Composer
                {accounts}
                {form}
                {sending}
                error={sendError}
                onchange={(next) => form = next}
                onsend={send}
                ondiscard={() => form = null}
            />
        </Modal>
    {/if}
{/if}

<style>
    .frame {
        position: relative;
        display: grid;
        grid-template-rows: auto auto minmax(0, 1fr);
        width: 100%;
        height: 100%;
        border: 1px solid var(--glass-border-outer);
        border-radius: var(--radius-window);
        background: var(--glass-shine), var(--glass-tint);
        backdrop-filter: blur(var(--glass-blur));
        -webkit-backdrop-filter: blur(var(--glass-blur));
        box-shadow: inset 0 0 0 1px var(--glass-border-inner), var(--glass-shadow);
        overflow: hidden;
    }

    .toolbar {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: var(--space-1);
        padding: var(--space-2);
    }

    .gap {
        width: var(--space-2);
    }

    .spacer {
        flex: 1;
    }

    .move {
        min-height: 26px;
        max-width: 140px;
        padding: var(--space-1) var(--space-2);
        border: 1px solid var(--control-border);
        border-radius: var(--radius-control);
        background: var(--control-face);
        color: var(--control-text);
        box-shadow: inset 0 0 0 1px var(--control-inner);
    }

    .move:disabled {
        opacity: 0.55;
    }

    .move option {
        background: var(--field-background);
        color: var(--text);
    }

    .error {
        padding: 0 var(--space-2) var(--space-2);
    }

    .error-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: var(--space-3);
    }

    small {
        display: block;
        margin-top: var(--space-1);
        color: var(--text-muted);
        overflow-wrap: anywhere;
    }

    .panes {
        grid-row: 3;
        display: grid;
        grid-template-columns: 220px minmax(240px, 340px) minmax(0, 1fr);
        min-height: 0;
        margin: 0 var(--space-2) var(--space-2);
        border: 1px solid var(--surface-border);
        border-radius: var(--radius-control);
        overflow: hidden;
    }

    .panes > :global(*) {
        min-height: 0;
    }

    .panes > :global(* + *) {
        border-left: 1px solid var(--surface-border);
    }

    @media (max-width: 860px) {
        .panes {
            grid-template-columns: 180px minmax(0, 1fr);
            grid-template-rows: minmax(0, 2fr) minmax(0, 3fr);
        }

        .panes > :global(:first-child) {
            grid-row: 1 / 3;
        }

        .panes > :global(:last-child) {
            border-top: 1px solid var(--surface-border);
        }
    }
</style>

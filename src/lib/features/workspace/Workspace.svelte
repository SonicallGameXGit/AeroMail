<script lang="ts">
    import { onMount } from 'svelte'
    import { toMailError } from '$lib/api'
    import type { Account, Api, AppSettings, Attachment, Folder, MailError, MailEvent, MessageContent, MessageSummary, NotificationTarget, ReplyMode } from '$lib/api'
    import Backdrop from '$lib/design/Backdrop.svelte'
    import Button from '$lib/design/Button.svelte'
    import GlassWindow from '$lib/design/GlassWindow.svelte'
    import Icon from '$lib/design/Icon.svelte'
    import MessageBox from '$lib/design/MessageBox.svelte'
    import Modal from '$lib/design/Modal.svelte'
    import Notice from '$lib/design/Notice.svelte'
    import Splitter from '$lib/design/Splitter.svelte'
    import { applyTheme, colorScheme } from '$lib/design/theme'
    import AccountSetup from '$lib/features/accounts/AccountSetup.svelte'
    import { newMailNotification } from '$lib/features/notifications/notification'
    import Settings from '$lib/features/settings/Settings.svelte'
    import { t } from '$lib/i18n'
    import AccountMenu from './AccountMenu.svelte'
    import Composer from './Composer.svelte'
    import {
        clampList,
        clampSidebar,
        defaultLayout,
        listRange,
        loadLayout,
        railWidth,
        saveLayout,
        sidebarRange,
        type Layout,
    } from './layout'
    import MessageActions from './MessageActions.svelte'
    import MessageList from './MessageList.svelte'
    import Reader from './Reader.svelte'
    import Sidebar from './Sidebar.svelte'
    import { emptyForm, errorMessage, formFromDraft, groupFolders, pictureKey, senderPicture, toOutgoing, type ComposeForm, type Pictures } from './workspace'

    let { api }: { api: Api } = $props()

    // How often the open folder is checked for new mail.
    const refreshEveryMs = 2 * 60 * 1000

    // null until the account list was loaded.
    let accounts = $state<Account[] | null>(null)
    let folders = $state<Record<number, Folder[]>>({})
    // The account whose folders are shown.
    let accountId = $state<number | null>(null)
    let selectedFolder = $state<Folder | null>(null)
    let messages = $state<MessageSummary[]>([])
    let selectedId = $state<number | null>(null)
    let content = $state<MessageContent | null>(null)
    let remoteAllowed = $state(false)

    let syncing = $state(false)
    let opening = $state(false)
    let error = $state<MailError | null>(null)

    let addingAccount = $state(false)
    let settingsOpen = $state(false)
    // The account whose removal is waiting for confirmation.
    let removing = $state<Account | null>(null)
    // The account just added, until its message box is closed.
    let added = $state<{ account: Account, opened: boolean } | null>(null)
    let layout = $state<Layout>(loadLayout())

    // Until the real settings were loaded: nothing is looked up or shown.
    let settings = $state<AppSettings>({
        senderPictures: false,
        remoteImages: false,
        switchToNewAccount: true,
        theme: 'System',
        notifications: {
            enabled: false,
            sound: false,
            whileFocused: false,
            duration: 'System',
            seconds: 10,
            title: '',
            body: '',
            previewLength: 90,
            icon: 'App',
            customIcon: null,
        },
    })
    let pictures = $state<Pictures>({})
    // Addresses already asked about, so a refreshed list asks nothing twice.
    // Not state on purpose: asking must not make the list ask again.
    let asked: Record<string, true> = {}
    let form = $state<ComposeForm | null>(null)
    let sending = $state(false)
    let sendError = $state<MailError | null>(null)

    let selected = $derived(messages.find((message) => message.id === selectedId) ?? null)
    let account = $derived(accounts?.find((candidate) => candidate.id === accountId) ?? null)
    let accountFolders = $derived(accountId === null ? [] : folders[accountId] ?? [])
    let archiveFolder = $derived(accountFolders.find((folder) => folder.role === 'Archive' && folder.selectable) ?? null)
    // Offered in the order the sidebar shows them.
    let moveTargets = $derived.by(() => {
        const groups = groupFolders(accountFolders)

        return [...groups.main, ...groups.custom.map((entry) => entry.folder)]
            .filter((folder) => folder.id !== selectedFolder?.id)
    })
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

    const selectAccount = async (next: Account) => {
        accountId = next.id
        selectedFolder = null
        messages = []
        selectedId = null
        content = null

        const list = (folders[next.id] ?? []).filter((folder) => folder.selectable)
        const first = list.find((folder) => folder.role === 'Inbox') ?? list[0]
        if (first !== undefined) {
            await selectFolder(first)
        }
    }

    // Pictures are decoration: they arrive when they arrive, and a failure
    // to get one is never shown as an error.
    $effect(() => {
        if (!settings.senderPictures) {
            return
        }

        for (const message of messages) {
            const sender = message.from[0]
            if (sender === undefined || asked[pictureKey(sender.email)] === true) {
                continue
            }
            const key = pictureKey(sender.email)
            asked[key] = true

            api.senderPicture(sender.email)
                .then((picture) => {
                    if (picture !== null && settings.senderPictures) {
                        pictures = { ...pictures, [key]: picture }
                    }
                })
                .catch(() => {
                    delete asked[key]
                })
        }
    })

    // Settings are written one after another: the core answers requests side
    // by side, and an earlier write that finished last would undo a later one.
    let settingsWrites: Promise<void> = Promise.resolve()

    const saveSettings = (next: AppSettings): Promise<void> => {
        const write = settingsWrites.then(() => api.setSettings(next))
        settingsWrites = write.catch(() => undefined)

        return write
    }

    const changeSettings = (next: AppSettings) => guard(async () => {
        const before = settings
        // Notification choices show at once: their texts change with every
        // key, and a field that waited for the core would lose what was typed.
        // The rest waits: pictures are asked for as soon as the switch shows on.
        const onlyNotifications = next.senderPictures === before.senderPictures
            && next.remoteImages === before.remoteImages
            && next.switchToNewAccount === before.switchToNewAccount
            && next.theme === before.theme
        if (onlyNotifications) {
            settings = next
        }
        try {
            await saveSettings(next)
        } catch (caught) {
            // What the core kept is what is shown.
            settings = await api.settings()
            throw caught
        }
        if (onlyNotifications) {
            return
        }
        if (next.senderPictures !== before.senderPictures) {
            asked = {}
            pictures = {}
        }
        settings = next

        // The open message is shown the way the new setting says.
        if (next.remoteImages !== before.remoteImages && selectedId !== null) {
            const id = selectedId
            const opened = await api.openMessage(id, false)
            if (selectedId === id) {
                content = opened
                remoteAllowed = false
            }
        }
    })

    // The theme is shown once the stored one is known: the shell keeps the
    // window hidden until then, so it never appears in the wrong one.
    let themeKnown = $state(false)

    $effect(() => {
        if (!themeKnown) {
            return
        }

        const root = document.documentElement
        applyTheme(root, settings.theme)
        api.setColorScheme(colorScheme(settings.theme)).catch(() => undefined)

        return () => applyTheme(root, 'System')
    })

    const load = () => guard(async () => {
        try {
            settings = await api.settings()
        } finally {
            themeKnown = true
        }
        const list = await api.listAccounts()
        for (const account of list) {
            folders = { ...folders, [account.id]: await api.listFolders(account.id) }
        }
        accounts = list

        // The open account stays open; the first one is opened otherwise.
        const open = list.find((candidate) => candidate.id === accountId) ?? list[0]
        if (open === undefined) {
            accountId = null
            settingsOpen = false
            return
        }
        if (open.id !== accountId || selectedFolder === null) {
            await selectAccount(open)
        }
    })

    const refresh = () => guard(async () => {
        if (selectedFolder === null) {
            return
        }
        const folder = selectedFolder
        // The open folder first: counting every folder takes the server seconds.
        await sync(folder, false)
        await api.refreshFolders(folder.accountId)
        await reloadFolders(folder.accountId)
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

    const toggleFlagged = (message: MessageSummary) => guard(async () => {
        const flagged = !message.flags.flagged
        await api.setFlagged([message.id], flagged)
        patch(message.id, (current) => ({ ...current, flags: { ...current.flags, flagged } }))
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

    const compose = () => {
        if (accountId === null) {
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

    const accountAdded = (account: Account) => {
        addingAccount = false
        // The very first account is opened whatever the setting says.
        const opened = settings.switchToNewAccount || accounts === null || accounts.length === 0
        // Asked before load(), which keeps the open account open.
        if (settings.switchToNewAccount) {
            // The settings would stand between the user and the account.
            settingsOpen = false
            accountId = account.id
            selectedFolder = null
        }
        // Shown at once: opening the account waits for its server.
        added = { account, opened }
        load()
    }

    const removeAccount = (gone: Account) => guard(async () => {
        removing = null
        await api.removeAccount(gone.id)
        if (accountId === gone.id) {
            accountId = null
            selectedFolder = null
            messages = []
            selectedId = null
            content = null
        }
        await load()
    })

    const resize = (change: Partial<Layout>) => {
        layout = { ...layout, ...change }
        saveLayout(layout)
    }

    // How long a notification waits for a sender's picture that is not known
    // yet; after that it shows the initial.
    const pictureWaitMs = 1000

    const pictureOf = async (email: string): Promise<string | null> => {
        const known = pictures[pictureKey(email)]
        if (known !== undefined) {
            return known
        }
        if (!settings.senderPictures) {
            return null
        }

        const late = new Promise<null>((resolve) => setTimeout(() => resolve(null), pictureWaitMs))

        return Promise.race([api.senderPicture(email), late])
    }

    const resetNotifications = () => guard(async () => {
        const defaults = await api.defaultSettings()
        await changeSettings({ ...settings, notifications: defaults.notifications })
    })

    // The core fetched something by itself (it watches every inbox): the
    // lists are read again from the cache, which needs no server.
    const heard = async (event: MailEvent) => {
        if (accounts === null || !accounts.some((candidate) => candidate.id === event.accountId)) {
            return
        }

        if (event.type === 'newMail') {
            const notification = await newMailNotification(event, settings.notifications, pictureOf)
            if (notification !== null) {
                // Only an extra: a notification that cannot be shown costs nothing else.
                api.notify(notification).catch(() => undefined)
            }

            return
        }

        await reloadFolders(event.accountId)
        if (selectedFolder?.id === event.folderId) {
            const fresh = await api.listMessages(event.folderId)
            // The user may have moved on while the cache was read.
            if (selectedFolder?.id === event.folderId) {
                messages = fresh
            }
        }
    }

    // A click on a notification leads to the message it was about.
    const openTarget = (target: NotificationTarget) => guard(async () => {
        const folder = folders[target.accountId]?.find((candidate) => candidate.id === target.folderId)
        if (folder === undefined) {
            return
        }

        settingsOpen = false
        accountId = target.accountId
        await selectFolder(folder)
        const message = messages.find((candidate) => candidate.id === target.messageId)
        if (message !== undefined) {
            await openMessage(message)
        }
    })

    onMount(() => {
        load()

        const stopListening = api.onEvent((event) => {
            heard(event).catch(() => undefined)
        })
        const stopOpening = api.onNotificationOpen((target) => {
            openTarget(target)
        })

        const timer = setInterval(() => {
            if (selectedFolder !== null && !syncing) {
                const folder = selectedFolder
                // A background check that fails stays quiet; the next one retries.
                sync(folder, false).catch(() => undefined)
            }
        }, refreshEveryMs)

        return () => {
            clearInterval(timer)
            stopListening()
            stopOpening()
        }
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
            <header class="top">
                <span class="brand">
                    <Icon name="mail" size={22} />
                    <span>{t('app.name')}</span>
                </span>
                <span class="spacer"></span>
                {#if account !== null}
                    <AccountMenu
                        {accounts}
                        {folders}
                        current={account}
                        onselect={(next) => guard(() => selectAccount(next))}
                        onadd={() => addingAccount = true}
                        onsettings={() => settingsOpen = true}
                    />
                {/if}
            </header>

            {#if error !== null}
                <div class="error">
                    <Notice kind="danger">
                        <span class="error-row">
                            <Icon name="warning" size={20} />
                            <span class="error-text">
                                {errorMessage(error)}
                                <small>{t('error.details')}: {error.detail}</small>
                            </span>
                            <Button onclick={() => error = null}>{t('app.dismiss')}</Button>
                        </span>
                    </Notice>
                </div>
            {/if}

            <div class="panes" style:--sidebar-width={layout.sidebar + 'px'} style:--list-width={layout.list + 'px'} style:--rail-width={railWidth + 'px'}>
                <div class="pane sidebar">
                    <Sidebar
                        folders={accountFolders}
                        selectedFolderId={selectedFolder?.id ?? null}
                        onselect={selectFolder}
                        oncompose={compose}
                    />
                </div>
                <Splitter
                    label={t('sidebar.resize')}
                    value={layout.sidebar}
                    min={sidebarRange.min}
                    max={sidebarRange.max}
                    onchange={(width) => resize({ sidebar: clampSidebar(width) })}
                    onreset={() => resize({ sidebar: defaultLayout.sidebar })}
                />
                <div class="pane list">
                    <MessageList
                        folder={selectedFolder}
                        {messages}
                        {selectedId}
                        {pictures}
                        {hasMore}
                        busy={syncing}
                        onselect={openMessage}
                        onflag={toggleFlagged}
                        onrefresh={refresh}
                        onmore={() => guard(async () => {
                            if (selectedFolder !== null) {
                                await sync(selectedFolder, true)
                            }
                        })}
                    />
                </div>
                <Splitter
                    label={t('list.resize')}
                    value={layout.list}
                    min={listRange.min}
                    max={listRange.max}
                    onchange={(width) => resize({ list: clampList(width) })}
                    onreset={() => resize({ list: defaultLayout.list })}
                />
                <div class="pane reader">
                    <Reader
                        {content}
                        busy={opening}
                        remoteAllowed={remoteAllowed || settings.remoteImages}
                        picture={content === null ? null : senderPicture(pictures, content.summary)}
                        toolbar={selected === null ? undefined : actions}
                        onshowremote={showRemote}
                        onsave={saveAttachment}
                        onlink={openLink}
                    />
                </div>
            </div>
        </div>
    </Backdrop>

    {#snippet actions()}
        {#if selected !== null}
            {@const message = selected}
            <MessageActions
                {message}
                canArchive={archiveFolder !== null}
                {moveTargets}
                onreply={reply}
                onarchive={() => archiveFolder !== null && moveTo(archiveFolder.id)}
                onmove={moveTo}
                ondelete={remove}
                onseen={toggleSeen}
                onflag={() => toggleFlagged(message)}
            />
        {/if}
    {/snippet}

    {#if settingsOpen}
        <Modal title={t('settings.title')} size="wide">
            <Settings
                {accounts}
                {settings}
                onchange={changeSettings}
                onadd={() => addingAccount = true}
                onremove={(next) => removing = next}
                onclose={() => settingsOpen = false}
                onresetnotifications={resetNotifications}
            />
        </Modal>
    {/if}

    {#if addingAccount}
        <Modal title={t('account.add')}>
            <AccountSetup {api} onadded={accountAdded} oncancel={() => addingAccount = false} />
        </Modal>
    {/if}

    {#if removing !== null}
        {@const gone = removing}
        <MessageBox
            title={t('account.remove')}
            icon="warning"
            heading={t('account.removeHeading', { email: gone.settings.email })}
            onescape={() => removing = null}
        >
            {t('account.removeHint')}
            {#snippet buttons()}
                <Button onclick={() => removing = null}>{t('account.removeNo')}</Button>
                <Button tone="danger" icon="trash" onclick={() => removeAccount(gone)}>{t('account.removeYes')}</Button>
            {/snippet}
        </MessageBox>
    {/if}

    {#if form !== null}
        <Modal title={t('compose.title')} size="wide">
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

<!-- Outside the workspace: after the very first account it shows while the
     workspace is still being opened. -->
{#if added !== null}
    <MessageBox
        title={t('account.added')}
        icon="success"
        heading={t('account.addedHeading', { email: added.account.settings.email })}
        onescape={() => added = null}
    >
        {added.opened ? t('account.addedOpened') : t('account.addedKept')}
        {#snippet buttons()}
            <Button variant="accent" onclick={() => added = null}>{t('app.ok')}</Button>
        {/snippet}
    </MessageBox>
{/if}

<style>
    .frame {
        position: relative;
        display: grid;
        grid-template-rows: auto auto minmax(0, 1fr);
        width: 100%;
        height: 100%;
        background: var(--glass-shine), var(--glass-tint);
        backdrop-filter: blur(var(--glass-blur));
        -webkit-backdrop-filter: blur(var(--glass-blur));
        overflow: hidden;
    }

    .top {
        display: flex;
        align-items: center;
        gap: var(--space-3);
        padding: var(--space-2);
        color: var(--glass-text);
    }

    .brand {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        padding-left: var(--space-1);
        text-shadow: var(--glass-text-shadow);
    }

    .spacer {
        flex: 1;
    }

    .error {
        padding: 0 var(--space-2) var(--space-2);
    }

    .error-row {
        display: flex;
        align-items: center;
        gap: var(--space-3);
    }

    .error-text {
        flex: 1;
    }

    small {
        display: block;
        margin-top: var(--space-1);
        color: var(--text-muted);
        overflow-wrap: anywhere;
    }

    .panes {
        grid-row: 3;
        display: flex;
        min-height: 0;
        margin: 0 var(--space-2) var(--space-2);
        border: 1px solid var(--surface-border);
        border-radius: var(--radius-control);
        overflow: hidden;
    }

    .pane {
        display: grid;
        min-width: 0;
        min-height: 0;
    }

    .sidebar {
        flex: none;
        width: var(--sidebar-width);
    }

    /* When the window is too narrow for the chosen widths, the list gives way before the reader does. */
    .list {
        flex: 0 1 var(--list-width);
        min-width: 200px;
    }

    .reader {
        flex: 1 0 320px;
    }

    @media (max-width: 860px) {
        .sidebar {
            width: var(--rail-width);
        }
    }
</style>

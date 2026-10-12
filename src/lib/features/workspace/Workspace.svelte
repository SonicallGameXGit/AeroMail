<script lang="ts">
    import { onMount, tick } from 'svelte'
    import { toMailError } from '$lib/api'
    import type { Account, Api, AppSettings, Attachment, Category, CategoryRule, CategorySettings, Conversation, ConversationSettings, Folder, MailError, MailEvent, MessageContent, MessageSummary, Moved, NotificationTarget, ReplyMode } from '$lib/api'
    import Backdrop from '$lib/design/Backdrop.svelte'
    import Button from '$lib/design/Button.svelte'
    import GlassWindow from '$lib/design/GlassWindow.svelte'
    import Icon from '$lib/design/Icon.svelte'
    import MessageBox from '$lib/design/MessageBox.svelte'
    import Modal from '$lib/design/Modal.svelte'
    import Notice from '$lib/design/Notice.svelte'
    import Splitter from '$lib/design/Splitter.svelte'
    import Toast from '$lib/design/Toast.svelte'
    import { applyTheme, colorScheme } from '$lib/design/theme'
    import AccountSetup from '$lib/features/accounts/AccountSetup.svelte'
    import { newMailNotification } from '$lib/features/notifications/notification'
    import Settings from '$lib/features/settings/Settings.svelte'
    import { t } from '$lib/i18n'
    import AccountMenu from './AccountMenu.svelte'
    import Composer from './Composer.svelte'
    import ConversationView from './ConversationView.svelte'
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
    import { emptySelection, keepListed, type Selection } from './selection'
    import Sidebar from './Sidebar.svelte'
    import {
        answerTarget,
        categoryUnread,
        emptyForm,
        errorMessage,
        folderLabel,
        formFromDraft,
        groupFolders,
        inCategory,
        nextRow,
        pictureKey,
        senderPicture,
        shownCategories,
        toOutgoing,
        type ComposeForm,
        type ListAction,
        type Pictures,
    } from './workspace'

    let { api }: { api: Api } = $props()

    // How often the open folder is checked for new mail.
    const refreshEveryMs = 2 * 60 * 1000

    // null until the account list was loaded.
    let accounts = $state<Account[] | null>(null)
    let folders = $state<Record<number, Folder[]>>({})
    // The account whose folders are shown.
    let accountId = $state<number | null>(null)
    let selectedFolder = $state<Folder | null>(null)
    // The rows of the open folder: conversations, or messages on their own.
    let conversations = $state<Conversation[]>([])
    // The message its row was opened by; the row is the one holding it.
    let selectedId = $state<number | null>(null)
    // What is open: one message, or a conversation of several (oldest first).
    let content = $state<MessageContent | null>(null)
    let thread = $state<MessageSummary[] | null>(null)
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
        reader: 'Beside',
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
        conversations: {
            enabled: false,
            sameSubject: false,
            otherFolders: false,
        },
        categories: {
            enabled: false,
            social: false,
            promotions: false,
            updates: false,
            forums: false,
        },
    })
    // The open tab of a sorted inbox, and the senders the user sorted by hand.
    let category = $state<Category>('Primary')
    let rules = $state<CategoryRule[]>([])
    let pictures = $state<Pictures>({})
    // Addresses already asked about, so a refreshed list asks nothing twice.
    // Not state on purpose: asking must not make the list ask again.
    let asked: Record<string, true> = {}
    // The ticked rows of the list.
    let selection = $state<Selection>(emptySelection)
    // The folder or tab dragged rows are over, as its `data-drop` names it.
    let dropTarget = $state<string | null>(null)
    // What the last action did, until its message is closed, with its undo.
    let notice = $state<{ id: number, text: string, undo: (() => void) | null } | null>(null)
    let notices = 0
    let list = $state<ReturnType<typeof MessageList> | null>(null)
    let form = $state<ComposeForm | null>(null)
    let sending = $state(false)
    let sendError = $state<MailError | null>(null)

    let selected = $derived(conversations.find((conversation) => selectedId !== null && conversation.messageIds.includes(selectedId)) ?? null)
    // Only the inbox is sorted, and only while the setting says so.
    let tabs = $derived.by(() => {
        if (selectedFolder?.role !== 'Inbox') {
            return []
        }

        return shownCategories(settings.categories).map((shown) => ({ category: shown, unread: categoryUnread(conversations, shown) }))
    })
    // A tab that was turned off is not open any more: its mail is in Primary.
    let openCategory = $derived(tabs.some((tab) => tab.category === category) ? category : 'Primary')
    let visible = $derived(tabs.length === 0 ? conversations : inCategory(conversations, openCategory))
    let shownSelection = $derived(keepListed(selection, visible.map((conversation) => conversation.latest.id)))
    // An open message covers the list instead of standing next to it.
    let full = $derived(settings.reader === 'Full')
    let reading = $derived(selectedId !== null)
    let account = $derived(accounts?.find((candidate) => candidate.id === accountId) ?? null)
    let accountFolders = $derived(accountId === null ? [] : folders[accountId] ?? [])
    // Archiving takes mail out of the inbox and keeps it: in the Archive
    // folder, or where the account keeps everything (Gmail's All mail). Only
    // from the inbox and the user's own folders.
    let archiveFolder = $derived.by(() => {
        const role = selectedFolder?.role
        if (role !== 'Inbox' && role !== 'Other' && role !== 'Flagged') {
            return null
        }
        const usable = accountFolders.filter((folder) => folder.selectable && folder.id !== selectedFolder?.id)

        return usable.find((folder) => folder.role === 'Archive') ?? usable.find((folder) => folder.role === 'All') ?? null
    })
    // Spam goes to Junk; in Junk, "not spam" takes it back to the inbox.
    let spamFolder = $derived.by(() => {
        const role = selectedFolder?.role === 'Junk' ? 'Inbox' : 'Junk'

        return accountFolders.find((folder) => folder.role === role && folder.selectable) ?? null
    })
    let spam = $derived<'report' | 'undo' | null>(spamFolder === null ? null : selectedFolder?.role === 'Junk' ? 'undo' : 'report')
    // Offered in the order the sidebar shows them.
    let moveTargets = $derived.by(() => {
        const groups = groupFolders(accountFolders)

        return [...groups.main, ...groups.custom.map((entry) => entry.folder)]
            .filter((folder) => folder.id !== selectedFolder?.id)
    })
    let listed = $derived(conversations.reduce((sum, conversation) => sum + conversation.messageIds.length, 0))
    let hasMore = $derived(selectedFolder !== null && listed < selectedFolder.total)

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

    const close = () => {
        selectedId = null
        content = null
        thread = null
    }

    // The rows of a folder, read again from the cache.
    const reloadList = async (folder: Folder) => {
        const rows = await api.listConversations(folder.id)
        // The user may have moved on to another folder in the meantime.
        if (selectedFolder?.id === folder.id) {
            conversations = rows
        }
    }

    // The open conversation, read again from the cache: a reply may have joined it.
    const reloadThread = async () => {
        const id = selectedId
        if (thread === null || id === null) {
            return
        }

        const fresh = await api.conversation(id)
        if (thread !== null && selectedId === id) {
            thread = fresh
        }
    }

    const sync = async (folder: Folder, grow: boolean) => {
        syncing = true
        try {
            await api.syncFolder(folder.id, grow)
            await reloadList(folder)
            if (selectedFolder?.id === folder.id && selectedId !== null && selected === null) {
                close()
            }
            await reloadFolders(folder.accountId)
        } finally {
            syncing = false
        }
    }

    const selectFolder = (folder: Folder) => guard(async () => {
        selectedFolder = folder
        selection = emptySelection
        close()
        // Cached messages show at once; the server is asked right after.
        await reloadList(folder)
        await sync(folder, false)
    })

    const selectAccount = async (next: Account) => {
        accountId = next.id
        selectedFolder = null
        conversations = []
        close()

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

        const shown = [...conversations.map((conversation) => conversation.latest), ...thread ?? []]
        for (const message of shown) {
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

    const sameCategories = (a: CategorySettings, b: CategorySettings): boolean => {
        return a.enabled === b.enabled && a.social === b.social && a.promotions === b.promotions
            && a.updates === b.updates && a.forums === b.forums
    }

    const sameGrouping = (a: ConversationSettings, b: ConversationSettings): boolean => {
        return a.enabled === b.enabled && a.sameSubject === b.sameSubject && a.otherFolders === b.otherFolders
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
            && next.reader === before.reader
            && sameGrouping(next.conversations, before.conversations)
            && sameCategories(next.categories, before.categories)
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
        if (next.remoteImages !== before.remoteImages && content !== null) {
            const id = content.summary.id
            const opened = await api.openMessage(id, false)
            if (selectedId === id) {
                content = opened
                remoteAllowed = false
            }
        }
        // The list is grouped anew, and what was open is opened the new way.
        if (!sameGrouping(next.conversations, before.conversations) || !sameCategories(next.categories, before.categories)) {
            if (selectedFolder !== null) {
                await reloadList(selectedFolder)
            }
        }
        if (!sameGrouping(next.conversations, before.conversations) && selectedFolder !== null) {
            await reloadList(selectedFolder)
            if (selected !== null) {
                await openConversation(selected)
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
            rules = await api.categoryRules()
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

    // Changes the flags of messages wherever they are shown open.
    const patch = (ids: number[], flags: Partial<MessageSummary['flags']>) => {
        const change = (message: MessageSummary): MessageSummary => {
            return ids.includes(message.id) ? { ...message, flags: { ...message.flags, ...flags } } : message
        }
        if (content !== null) {
            content = { ...content, summary: change(content.summary) }
        }
        if (thread !== null) {
            thread = thread.map(change)
        }
    }

    // Resolves to the messages it changed.
    const setSeen = async (ids: number[], seen: boolean): Promise<number[]> => {
        const changed = await api.setSeen(ids, seen)
        patch(ids, { seen })
        if (selectedFolder !== null) {
            await reloadList(selectedFolder)
            await reloadFolders(selectedFolder.accountId)
        }

        return changed
    }

    const setStarred = async (ids: number[], flagged: boolean): Promise<number[]> => {
        const changed = await api.setFlagged(ids, flagged)
        patch(ids, { flagged })
        if (selectedFolder !== null) {
            await reloadList(selectedFolder)
        }

        return changed
    }

    // A conversation of several opens as a chat; a message on its own as before.
    // Opening it reads it: everything unread in it is marked read.
    const openConversation = (conversation: Conversation) => guard(async () => {
        const message = conversation.latest
        selectedId = message.id
        content = null
        thread = null
        remoteAllowed = false
        opening = true
        try {
            if (conversation.count > 1) {
                const opened = await api.conversation(message.id)
                if (selectedId !== message.id) {
                    return
                }
                thread = opened
                const unread = opened.filter((candidate) => !candidate.flags.seen).map((candidate) => candidate.id)
                if (unread.length > 0) {
                    await setSeen(unread, true)
                }

                return
            }

            const opened = await api.openMessage(message.id, false)
            if (selectedId !== message.id) {
                return
            }
            content = opened
            if (!message.flags.seen) {
                await setSeen([message.id], true)
            }
        } finally {
            opening = false
        }
    })

    const showRemote = () => guard(async () => {
        if (content === null) {
            return
        }
        const id = content.summary.id
        const opened = await api.openMessage(id, true)
        if (selectedId === id) {
            content = opened
            remoteAllowed = true
        }
    })

    // After rows leave the list, the row below them in the open tab takes
    // the open one's place, as it does in Gmail.
    const leave = async (rows: Conversation[]) => {
        const open = selected
        const wasOpen = open !== null && rows.some((row) => row.latest.id === open.latest.id)
        const next = wasOpen ? nextRow(visible, rows) : null
        const gone = rows.map((row) => row.latest.id)
        conversations = conversations.filter((conversation) => !gone.includes(conversation.latest.id))
        selection = { ...selection, keys: selection.keys.filter((key) => !gone.includes(key)) }
        if (wasOpen) {
            close()
        }
        if (selectedFolder !== null) {
            await reloadFolders(selectedFolder.accountId)
        }
        if (next !== null) {
            await openConversation(next)
        }
    }

    // How the message of an action names what it acted on.
    const what = (rows: Conversation[]): string => {
        const grouped = settings.conversations.enabled
        if (rows.length === 1) {
            return grouped ? t('undo.conversation') : t('undo.message')
        }
        const count = String(rows.length)

        return grouped ? t('undo.conversations', { count }) : t('undo.messages', { count })
    }

    // The list as it was before an action, to come back to when it is undone.
    type Snapshot = {
        folderId: number
        selection: Selection
        openId: number | null
        category: Category
    }

    // Undoing resolves to the new ids of messages that came back under one.
    type Undo = () => Promise<Map<number, number>>

    const say = (text: string, undo: (() => void) | null = null) => {
        notices += 1
        notice = { id: notices, text, undo }
    }

    const offer = (text: string, undo: Undo | null, snapshot: Snapshot) => {
        say(text, undo === null ? null : () => revert(undo, snapshot))
    }

    // Undoes an action and brings back what was ticked and open.
    const revert = (undo: Undo, snapshot: Snapshot) => guard(async () => {
        notice = null
        const renamed = await undo()
        say(t('undo.undone'))
        const folder = selectedFolder
        if (folder === null) {
            return
        }
        await reloadList(folder)
        await reloadFolders(folder.accountId)
        if (folder.id !== snapshot.folderId) {
            return
        }

        const rowOf = (id: number | null): number | null => {
            if (id === null) {
                return null
            }
            const now = renamed.get(id) ?? id

            return conversations.find((conversation) => conversation.messageIds.includes(now))?.latest.id ?? null
        }
        const rows = (keys: number[]) => keys.map(rowOf).filter((key) => key !== null)
        category = snapshot.category
        selection = {
            keys: rows(snapshot.selection.keys),
            anchor: rowOf(snapshot.selection.anchor),
            base: rows(snapshot.selection.base),
        }
        const openRow = rowOf(snapshot.openId)
        const reopen = conversations.find((conversation) => conversation.latest.id === openRow)
        if (reopen !== undefined && (selected === null || selected.latest.id !== openRow)) {
            await openConversation(reopen)
        }
    })

    const putBack = async (moves: Moved[]): Promise<Map<number, number>> => {
        const restored = await api.putBack(moves)

        return new Map(restored.map((entry) => [entry.was, entry.now]))
    }

    const sortedText = (senders: string[], first: string, next: Category | null): string => {
        if (next === null) {
            return t('undo.categoryAuto', { sender: first })
        }
        if (senders.length === 1) {
            return t('undo.category', { sender: first, name: t(`category.${next}`) })
        }

        return t('undo.categoryMany', { count: String(senders.length), name: t(`category.${next}`) })
    }

    // Where archiving, reporting spam or moving takes rows.
    const targetOf = (action: ListAction): Folder | null => {
        if (action.kind === 'archive') {
            return archiveFolder
        }
        if (action.kind === 'spam') {
            return spamFolder
        }
        if (action.kind === 'move') {
            return accountFolders.find((candidate) => candidate.id === action.folderId) ?? null
        }

        return null
    }

    const movedText = (action: ListAction, target: Folder, named: string): string => {
        if (action.kind === 'archive') {
            return t('undo.archived', { what: named })
        }
        if (action.kind === 'spam' && target.role === 'Junk') {
            return t('undo.spam', { what: named })
        }

        return t('undo.moved', { what: named, folder: folderLabel(target) })
    }

    // Every message from the rows' senders goes to `next` from now on; null
    // gives them back to the automatic rules.
    const sortSenders = async (rows: Conversation[], next: Category | null, snapshot: Snapshot) => {
        const senders = [...new Set(rows.flatMap((row) => {
            const sender = row.latest.from[0]?.email.trim().toLowerCase()

            return sender === undefined || sender === '' ? [] : [sender]
        }))]
        const first = senders[0]
        if (first === undefined || selectedFolder === null) {
            return
        }
        const folder = selectedFolder
        const before = senders.map((sender) => ({ sender, category: rules.find((rule) => rule.sender === sender)?.category ?? null }))
        for (const sender of senders) {
            await api.setCategoryRule(sender, next)
        }
        rules = await api.categoryRules()
        await reloadList(folder)

        offer(sortedText(senders, first, next), async () => {
            for (const rule of before) {
                await api.setCategoryRule(rule.sender, rule.category)
            }
            rules = await api.categoryRules()

            return new Map()
        }, snapshot)
    }

    // Everything done to rows of the list, wherever it was asked for: their
    // tools, the selection, the keyboard, a drop, the open message's toolbar.
    // Each says what it did and can be undone, except deleting for good.
    const act = (action: ListAction, rows: Conversation[]) => guard(async () => {
        const folder = selectedFolder
        if (folder === null || rows.length === 0) {
            return
        }
        const snapshot: Snapshot = { folderId: folder.id, selection: shownSelection, openId: selectedId, category: openCategory }
        const named = what(rows)
        // Deleting or moving a conversation takes all of its messages in this folder.
        const ids = rows.flatMap((row) => row.messageIds)
        const newest = rows.map((row) => row.latest.id)

        if (action.kind === 'delete') {
            const moves = await api.deleteMessages(ids)
            await leave(rows)
            const kept = moves.filter((entry) => entry.toFolderId !== null)
            if (kept.length === 0) {
                say(t('undo.deletedForGood', { what: named }))

                return
            }
            offer(t('undo.deleted', { what: named }), () => putBack(kept), snapshot)

            return
        }
        if (action.kind === 'archive' || action.kind === 'spam' || action.kind === 'move') {
            const target = targetOf(action)
            if (target === null || target.id === folder.id) {
                return
            }
            const moves = await api.moveMessages(ids, target.id)
            await leave(rows)
            offer(movedText(action, target, named), () => putBack(moves), snapshot)

            return
        }
        if (action.kind === 'seen') {
            // Read when everything is read: marking unread marks the newest
            // message, which is enough to bring it back to attention.
            const changed = await setSeen(action.seen ? ids : newest, action.seen)
            offer(t(action.seen ? 'undo.read' : 'undo.unread', { what: named }), async () => {
                await setSeen(changed, !action.seen)

                return new Map()
            }, snapshot)

            return
        }
        if (action.kind === 'star') {
            // Starring stars the newest message; removing the star clears every star in it.
            const changed = await setStarred(action.star ? newest : ids, action.star)
            offer(t(action.star ? 'undo.starred' : 'undo.unstarred', { what: named }), async () => {
                await setStarred(changed, !action.star)

                return new Map()
            }, snapshot)

            return
        }

        await sortSenders(rows, action.category, snapshot)
    })

    // The open conversation's sender, from its toolbar: the tab follows it there.
    const sortSender = (next: Category | null) => guard(async () => {
        const open = selected
        if (open === null || selectedFolder === null) {
            return
        }
        const snapshot: Snapshot = { folderId: selectedFolder.id, selection: shownSelection, openId: selectedId, category: openCategory }
        await sortSenders([open], next, snapshot)
        if (next !== null) {
            category = next
        }
    })

    // Back from a message covering the list to the list as it was left.
    const back = async () => {
        close()
        await tick()
        list?.focusRow()
    }

    // Escape inside the message goes back too, unless it closes something else first.
    const readerKey = (event: KeyboardEvent) => {
        const typing = event.target instanceof HTMLElement && event.target.closest('input, textarea, select, [contenteditable]') !== null
        if (!full || event.key !== 'Escape' || event.defaultPrevented || typing || document.querySelector(':popover-open') !== null) {
            return
        }
        event.preventDefault()
        back()
    }

    // Ctrl+Z undoes what the message at the bottom offers to undo.
    const windowKey = (event: KeyboardEvent) => {
        const typing = event.target instanceof HTMLElement && event.target.closest('input, textarea, select, [contenteditable]') !== null
        const undo = notice?.undo ?? null
        if (undo === null || typing || event.key !== 'z' || !(event.ctrlKey || event.metaKey) || event.shiftKey) {
            return
        }
        event.preventDefault()
        undo()
    }

    const forgetRule = (sender: string) => guard(async () => {
        await api.setCategoryRule(sender, null)
        rules = await api.categoryRules()
        if (selectedFolder !== null) {
            await reloadList(selectedFolder)
        }
    })

    const compose = () => {
        if (accountId === null) {
            return
        }
        sendError = null
        form = emptyForm(accountId)
    }

    // The composer, opened on a reply to `message` that starts with `text`.
    const startReply = async (message: MessageSummary, mode: ReplyMode, text = '') => {
        if (selectedFolder === null) {
            return
        }
        const draft = await api.replyDraft(message.id, mode)
        const started = formFromDraft(selectedFolder.accountId, draft)
        sendError = null
        form = { ...started, body: text + started.body }
    }

    // What the toolbar answers: the open message, or in a conversation the
    // newest message someone else wrote.
    const replyTarget = (): MessageSummary | null => {
        if (thread !== null && account !== null) {
            return answerTarget(thread, account.settings.email)
        }

        return content?.summary ?? selected?.latest ?? null
    }

    const reply = (mode: ReplyMode) => guard(async () => {
        const target = replyTarget()
        if (target !== null) {
            await startReply(target, mode)
        }
    })

    // What is written under a conversation goes to everyone in it, as an
    // answer to its newest message from someone else. A failure is shown by
    // the conversation, next to what was written.
    const answer = async (text: string) => {
        const target = replyTarget()
        if (target === null || selectedFolder === null) {
            return
        }
        const folder = selectedFolder
        const draft = await api.replyDraft(target.id, 'ReplyAll')
        const started = formFromDraft(folder.accountId, draft)
        await api.sendMessage(toOutgoing({ ...started, body: text + started.body }))

        // The core has the copy in Sent by now: it joins the conversation.
        await reloadThread()
        await reloadList(folder)
    }

    const expand = (text: string) => guard(async () => {
        const target = replyTarget()
        if (target !== null) {
            await startReply(target, 'ReplyAll', text)
        }
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
            } else if (selectedFolder !== null) {
                // A reply joins its conversation.
                await reloadList(selectedFolder)
                await reloadThread()
            }
        } catch (caught) {
            sendError = toMailError(caught)
        } finally {
            sending = false
        }
    }

    const saveAttachment = (message: MessageSummary, attachment: Attachment) => guard(async () => {
        await api.saveAttachment(message.id, attachment)
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
            conversations = []
            close()
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
        // A conversation takes messages from any folder of its account.
        if (selectedFolder !== null && selectedFolder.accountId === event.accountId) {
            await reloadList(selectedFolder)
            await reloadThread()
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
        const conversation = conversations.find((candidate) => candidate.messageIds.includes(target.messageId))
        if (conversation !== undefined) {
            await openConversation(conversation)
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
                        {dropTarget}
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
                <!-- List and message: side by side, or the message over the list,
                     which stays laid out underneath so it is found as it was left. -->
                <div class="stage" class:full class:reading>
                    <div class="pane list" inert={full && reading}>
                        <MessageList
                            bind:this={list}
                            folder={selectedFolder}
                            conversations={visible}
                            {tabs}
                            category={openCategory}
                            oncategory={(next) => category = next}
                            {selectedId}
                            selection={shownSelection}
                            ownEmail={account?.settings.email ?? ''}
                            {pictures}
                            {hasMore}
                            busy={syncing}
                            canArchive={archiveFolder !== null}
                            {spam}
                            {moveTargets}
                            onselect={openConversation}
                            onselection={(next) => selection = next}
                            onact={act}
                            ondragover={(target) => dropTarget = target}
                            onrefresh={refresh}
                            onmore={() => guard(async () => {
                                if (selectedFolder !== null) {
                                    await sync(selectedFolder, true)
                                }
                            })}
                        />
                    </div>
                    {#if !full}
                        <Splitter
                            label={t('list.resize')}
                            value={layout.list}
                            min={listRange.min}
                            max={listRange.max}
                            onchange={(width) => resize({ list: clampList(width) })}
                            onreset={() => resize({ list: defaultLayout.list })}
                        />
                    {/if}
                    {#if !full || reading}
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <div class="pane reader" onkeydown={readerKey}>
                            {#if thread !== null && account !== null}
                                <ConversationView
                                    {api}
                                    {thread}
                                    ownEmail={account.settings.email}
                                    {pictures}
                                    remoteImages={settings.remoteImages}
                                    toolbar={actions}
                                    onreply={(message, mode) => guard(() => startReply(message, mode))}
                                    onanswer={answer}
                                    onexpand={expand}
                                    onsave={saveAttachment}
                                    onlink={openLink}
                                />
                            {:else}
                                <Reader
                                    {content}
                                    busy={opening}
                                    remoteAllowed={remoteAllowed || settings.remoteImages}
                                    picture={content === null ? null : senderPicture(pictures, content.summary)}
                                    toolbar={selected === null ? undefined : actions}
                                    onshowremote={showRemote}
                                    onsave={(attachment) => content !== null && saveAttachment(content.summary, attachment)}
                                    onlink={openLink}
                                />
                            {/if}
                        </div>
                    {/if}
                </div>
            </div>
        </div>
    </Backdrop>

    {#snippet actions()}
        {#if selected !== null}
            {@const conversation = selected}
            <MessageActions
                seen={conversation.unread === 0}
                starred={conversation.flagged}
                canArchive={archiveFolder !== null}
                {spam}
                {moveTargets}
                categories={tabs.map((tab) => tab.category)}
                category={conversation.category}
                sender={conversation.latest.from[0]?.email ?? ''}
                sorted={rules.some((rule) => rule.sender === conversation.latest.from[0]?.email.trim().toLowerCase())}
                oncategory={sortSender}
                onreply={reply}
                onarchive={() => act({ kind: 'archive' }, [conversation])}
                onmove={(folderId) => act({ kind: 'move', folderId }, [conversation])}
                ondelete={() => act({ kind: 'delete' }, [conversation])}
                onspam={() => act({ kind: 'spam' }, [conversation])}
                onseen={() => act({ kind: 'seen', seen: conversation.unread > 0 }, [conversation])}
                onstar={() => act({ kind: 'star', star: !conversation.flagged }, [conversation])}
                onback={full ? back : undefined}
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
                {rules}
                onforgetrule={forgetRule}
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

<svelte:window onkeydown={windowKey} />

{#if notice !== null}
    {@const shown = notice}
    {#key shown.id}
        <Toast
            text={shown.text}
            action={shown.undo === null ? undefined : { label: t('undo.undo'), run: shown.undo }}
            closeLabel={t('undo.close')}
            onclose={() => notice = null}
        />
    {/key}
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

    /* Beside each other the stage is not a box of its own: list, splitter
       and reader share the row with the sidebar. */
    .stage {
        display: contents;
    }

    /* When the window is too narrow for the chosen widths, the list gives way before the reader does. */
    .list {
        flex: 0 1 var(--list-width);
        min-width: 200px;
    }

    .reader {
        flex: 1 0 320px;
    }

    /* Over each other: one cell, the message on top of the list. */
    .stage.full {
        flex: 1;
        display: grid;
        min-width: 0;
        min-height: 0;
    }

    .full > .pane {
        grid-area: 1 / 1;
    }

    .full.reading > .list {
        visibility: hidden;
    }

    @media (max-width: 860px) {
        .sidebar {
            width: var(--rail-width);
        }
    }
</style>

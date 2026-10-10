// The desktop shell. It stays thin (AGENTS.md 5.2): a window, the mail core as
// a child process, and the native pieces the page cannot do itself.

import { statSync } from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'
import { app, BrowserWindow, dialog, ipcMain, Menu, nativeTheme, net, protocol, shell } from 'electron'
import { createCore, type Reply } from './core.js'
import { parseNotification, showNotification } from './notifications.js'
import { createTray } from './tray.js'

// The built UI is served from app://mailclient/ instead of file://, so its
// absolute paths work and it gets a real origin.
const uiOrigin = 'app://mailclient'
// Set by `pnpm dev` to load the UI from the Vite dev server instead.
const devUrl = process.env['MAILCLIENT_DEV_URL']
// Lets tests run against a throwaway folder instead of the user's mail.
const dataDirectory = process.env['MAILCLIENT_DATA_DIR']

const externalSchemes = [
    'http:',
    'https:',
    'mailto:',
]

const coreBinary = (): string => {
    const name = process.platform === 'win32' ? 'mailclient-core.exe' : 'mailclient-core'
    if (app.isPackaged) {
        return path.join(process.resourcesPath, name)
    }

    return path.join(app.getAppPath(), 'target', 'debug', name)
}

const core = createCore(coreBinary(), dataDirectory === undefined ? [] : ['--data-dir', dataDirectory])

const iconsDirectory = (): string => path.join(app.getAppPath(), 'electron', 'icons')

const iconFile = (): string => path.join(iconsDirectory(), 'icon.png')

// The page's --backdrop-middle, so the window is not white before the page paints.
const backdropColor = (): string => nativeTheme.shouldUseDarkColors ? '#25323f' : '#7fd0f0'

// The window waits for the page to say which theme it shows (`shell:set-color-scheme`),
// but not forever: a page that never says still has to be seen.
const showAtLatest = 3000

// Closing the window only hides it; set once the app is really on its way out.
let quitting = false
// Whether the window has been on screen yet (it starts hidden, see `showAtLatest`).
let shownOnce = false

const failure = (kind: string, detail: string): Reply => {
    return { ok: false, error: { kind, server: null, detail } }
}

const isFile = (file: string): boolean => {
    try {
        return statSync(file).isFile()
    } catch {
        return false
    }
}

const serveUi = () => {
    const root = path.join(app.getAppPath(), 'build')
    const fallback = path.join(root, 'index.html')

    protocol.handle('app', (request) => {
        const requested = path.join(root, decodeURIComponent(new URL(request.url).pathname))
        // Anything that is not a file of the build is a route of the SPA.
        const inside = requested.startsWith(root + path.sep)
        const file = inside && isFile(requested) ? requested : fallback

        return net.fetch(pathToFileURL(file).toString())
    })
}

const createWindow = (): BrowserWindow => {
    const window = new BrowserWindow({
        title: 'MailClient',
        width: 1180,
        height: 760,
        minWidth: 720,
        minHeight: 480,
        icon: iconFile(),
        backgroundColor: backdropColor(),
        show: false,
        autoHideMenuBar: true,
        webPreferences: {
            preload: path.join(import.meta.dirname, 'preload.cjs'),
            contextIsolation: true,
            sandbox: true,
            nodeIntegration: false,
        },
    })

    // The page never navigates away or opens windows; links go through `openExternal`.
    window.webContents.setWindowOpenHandler(() => ({ action: 'deny' }))
    window.webContents.on('will-navigate', (event) => event.preventDefault())

    window.once('show', () => {
        shownOnce = true
    })
    // The app keeps running in the tray. The page stays loaded, hidden, because
    // it is what words the new-mail notifications.
    window.on('close', (event) => {
        if (!quitting) {
            event.preventDefault()
            window.hide()
        }
    })
    // Windows ends a session without `before-quit`; a hidden window must not hold up the shutdown.
    window.on('session-end', () => {
        quitting = true
    })

    window.loadURL(devUrl ?? uiOrigin + '/')
    setTimeout(() => {
        if (!window.isDestroyed() && !window.isVisible()) {
            window.show()
        }
    }, showAtLatest)

    return window
}

// Brings the window back from the tray, the taskbar or behind other windows.
const showWindow = () => {
    const window = BrowserWindow.getAllWindows()[0]
    if (window === undefined) {
        createWindow()

        return
    }

    if (window.isMinimized()) {
        window.restore()
    }
    window.show()
    window.focus()
}

const registerHandlers = () => {
    ipcMain.handle('core:call', (_event, command: unknown, params: unknown): Promise<Reply> | Reply => {
        if (typeof command !== 'string') {
            return failure('Invalid', 'the command is not a string')
        }

        return core.call(command, params)
    })

    ipcMain.handle('shell:save-attachment', async (event, messageId: unknown, index: unknown, name: unknown): Promise<Reply> => {
        const window = BrowserWindow.fromWebContents(event.sender)
        if (window === null || typeof name !== 'string') {
            return failure('Invalid', 'the attachment cannot be saved from here')
        }

        const choice = await dialog.showSaveDialog(window, { defaultPath: name })
        if (choice.canceled || choice.filePath === '') {
            return { ok: true, value: false }
        }

        const saved = await core.call('save_attachment', { messageId, index, path: choice.filePath })

        return saved.ok ? { ok: true, value: true } : saved
    })

    ipcMain.handle('shell:notify', (event, request: unknown): Reply => {
        const parsed = parseNotification(request)
        const window = BrowserWindow.fromWebContents(event.sender)
        if (parsed === null || window === null) {
            return failure('Invalid', 'this notification cannot be shown')
        }
        showNotification(window, parsed, iconFile(), (target) => {
            window.webContents.send('shell:notification-open', target)
        })

        return { ok: true, value: null }
    })

    ipcMain.handle('shell:set-color-scheme', (event, scheme: unknown): Reply => {
        const window = BrowserWindow.fromWebContents(event.sender)
        if (window === null || (scheme !== 'system' && scheme !== 'light' && scheme !== 'dark')) {
            return failure('Invalid', 'this colour scheme does not exist')
        }
        // Also what the page's prefers-color-scheme answers, so "system" in the page follows it.
        nativeTheme.themeSource = scheme
        window.setBackgroundColor(backdropColor())
        if (!window.isVisible()) {
            window.show()
        }

        return { ok: true, value: null }
    })

    ipcMain.handle('shell:open-external', async (_event, url: unknown): Promise<Reply> => {
        if (typeof url !== 'string' || !URL.canParse(url) || !externalSchemes.includes(new URL(url).protocol)) {
            return failure('Invalid', 'this link cannot be opened')
        }
        await shell.openExternal(url)

        return { ok: true, value: null }
    })
}

protocol.registerSchemesAsPrivileged([
    { scheme: 'app', privileges: { standard: true, secure: true, supportFetchAPI: true } },
])

// A second start only brings the running window forward, from the tray too:
// two cores must not share one database.
if (app.requestSingleInstanceLock()) {
    app.on('second-instance', () => showWindow())
    // macOS: a click on the Dock icon of an app whose window is closed. It also
    // comes at launch, while the window still waits for the page's theme.
    app.on('activate', () => {
        if (shownOnce) {
            showWindow()
        }
    })

    app.on('before-quit', () => {
        quitting = true
    })
    // Windows are only really closed while quitting.
    app.on('window-all-closed', () => app.quit())
    app.on('will-quit', () => core.stop())

    app.whenReady().then(() => {
        // The default menu is a development tool; macOS needs it for the standard shortcuts.
        if (app.isPackaged && process.platform !== 'darwin') {
            Menu.setApplicationMenu(null)
        }
        // Windows shows no notification of an app without this name.
        app.setAppUserModelId('app.mailclient.desktop')
        serveUi()
        registerHandlers()
        core.onEvent((event) => {
            for (const window of BrowserWindow.getAllWindows()) {
                window.webContents.send('core:event', event)
            }
        })
        core.start()
        createWindow()
        createTray(iconsDirectory(), { open: showWindow, quit: () => app.quit() })
    })
} else {
    app.quit()
}

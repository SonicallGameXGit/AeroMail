// The desktop shell. It stays thin (AGENTS.md 5.2): a window, the mail core as
// a child process, and the native pieces the page cannot do itself.

import { statSync } from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'
import { app, BrowserWindow, dialog, ipcMain, Menu, net, protocol, shell } from 'electron'
import { createCore, type Reply } from './core.js'

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
        icon: path.join(app.getAppPath(), 'electron', 'icons', 'icon.png'),
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

    window.loadURL(devUrl ?? uiOrigin + '/')

    return window
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

// A second start only brings the running window forward: two cores must not share one database.
if (app.requestSingleInstanceLock()) {
    app.on('second-instance', () => {
        const window = BrowserWindow.getAllWindows()[0]
        if (window !== undefined) {
            if (window.isMinimized()) {
                window.restore()
            }
            window.focus()
        }
    })

    app.on('window-all-closed', () => app.quit())
    app.on('will-quit', () => core.stop())

    app.whenReady().then(() => {
        // The default menu is a development tool; macOS needs it for the standard shortcuts.
        if (app.isPackaged && process.platform !== 'darwin') {
            Menu.setApplicationMenu(null)
        }
        serveUi()
        registerHandlers()
        createWindow()
    })
} else {
    app.quit()
}

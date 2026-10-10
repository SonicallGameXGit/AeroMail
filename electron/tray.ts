// The tray icon: the way back to the window once it is closed, while the app
// keeps watching for new mail. What differs between systems is handled here only.

import path from 'node:path'
import { Menu, nativeImage, Tray, type NativeImage } from 'electron'

export type TrayActions = {
    open: () => void
    quit: () => void
}

// Held for the whole run: a tray icon that is garbage collected disappears.
let tray: Tray | null = null

const trayImage = (iconsDirectory: string): NativeImage => {
    // Windows picks the right size out of the .ico by itself.
    if (process.platform === 'win32') {
        return nativeImage.createFromPath(path.join(iconsDirectory, 'icon.ico'))
    }

    const source = nativeImage.createFromPath(path.join(iconsDirectory, 'icon.png'))
    if (process.platform === 'darwin') {
        // The menu bar is 16-18 points high, drawn at twice that on Retina screens.
        const image = nativeImage.createEmpty()
        image.addRepresentation({ scaleFactor: 1, buffer: source.resize({ width: 18, height: 18 }).toPNG() })
        image.addRepresentation({ scaleFactor: 2, buffer: source.resize({ width: 36, height: 36 }).toPNG() })

        return image
    }

    // Linux tray hosts scale the picture to their panel themselves.
    return source.resize({ width: 64, height: 64 })
}

// Some Linux desktops (GNOME without an AppIndicator extension) show no tray
// at all and do not say so; the app then keeps running unseen, and starting
// it again brings the window back (single instance, in main.ts).
export const createTray = (iconsDirectory: string, actions: TrayActions) => {
    if (tray !== null) {
        return
    }

    tray = new Tray(trayImage(iconsDirectory))
    tray.setToolTip('MailClient')
    // Linux tray hosts often open the menu on any click, so opening the window is in it too.
    tray.setContextMenu(Menu.buildFromTemplate([
        { label: 'Open MailClient', click: actions.open },
        { type: 'separator' },
        { label: 'Quit MailClient', click: actions.quit },
    ]))
    tray.on('click', () => actions.open())
}

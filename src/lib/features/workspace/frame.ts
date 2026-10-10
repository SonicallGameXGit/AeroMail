// What the app does from outside the isolated message frame. The frame runs
// no scripts, so everything that needs code is done here, on its document.

import { isSafeLink } from './workspace'

// Links are caught and handed to the system browser instead of navigating
// inside the app. Call it from the frame's `load`.
export const catchLinks = (frame: HTMLIFrameElement, onlink: (url: string) => void) => {
    const document = frame.contentDocument
    // Elements inside the frame belong to the frame's own window, so they
    // must be checked against its classes, not the app's.
    const view = document?.defaultView ?? null
    if (document === null || view === null) {
        return
    }

    document.addEventListener('click', (click) => {
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

// Makes the frame as tall as the message in it, so a message among others
// scrolls with them instead of inside a box of its own. It follows the
// message as its pictures and fonts arrive and as the frame gets narrower or
// wider. Call it from the frame's `load`; gives what stops following.
export const fitHeight = (frame: HTMLIFrameElement): (() => void) => {
    const document = frame.contentDocument
    if (document === null) {
        return () => undefined
    }

    const fit = () => {
        // Measured from nothing: a frame never reports less than its own height.
        frame.style.height = '0px'
        frame.style.height = document.documentElement.scrollHeight + 'px'
    }
    fit()

    // Pictures announce themselves to the document on the way in.
    document.addEventListener('load', fit, true)
    document.fonts.ready.then(fit).catch(() => undefined)

    let width = frame.clientWidth
    const resized = new ResizeObserver(() => {
        if (frame.clientWidth !== width) {
            width = frame.clientWidth
            fit()
        }
    })
    resized.observe(frame)

    return () => {
        resized.disconnect()
        document.removeEventListener('load', fit, true)
    }
}

// Avatars as pictures, for places that cannot show the page's own avatar:
// a system notification gets a PNG. Drawn from the same tokens as Avatar.svelte.

import { avatarInitial, avatarTone } from './avatar'

// Notification services show icons at about 48-64 px; twice that stays sharp.
const avatarPixels = 128
// What a picture chosen by the user is made into before it is kept.
export const iconPixels = 128

const token = (name: string): string => {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

const canvasOf = (pixels: number): { canvas: HTMLCanvasElement, context: CanvasRenderingContext2D } | null => {
    const canvas = document.createElement('canvas')
    canvas.width = pixels
    canvas.height = pixels
    const context = canvas.getContext('2d')
    if (context === null) {
        return null
    }

    return { canvas, context }
}

const loadImage = async (url: string): Promise<HTMLImageElement | null> => {
    const image = new Image()
    image.src = url
    try {
        await image.decode()
    } catch {
        return null
    }

    return image
}

// The round avatar of a sender as a PNG `data:` address: their picture when
// there is one that can be drawn, their coloured initial otherwise.
export const avatarImage = async (name: string, seed: string, picture: string | null): Promise<string | null> => {
    const drawing = canvasOf(avatarPixels)
    if (drawing === null) {
        return null
    }
    const { canvas, context } = drawing
    const size = avatarPixels
    const middle = size / 2

    context.save()
    context.beginPath()
    context.arc(middle, middle, middle - 1, 0, Math.PI * 2)
    context.clip()

    const image = picture === null ? null : await loadImage(picture)
    if (image === null) {
        context.fillStyle = token(`--avatar-${avatarTone(seed)}`)
        context.fillRect(0, 0, size, size)
        context.fillStyle = token('--avatar-text')
        context.font = `600 ${Math.round(size * 0.5)}px ${token('--font-ui')}`
        context.textAlign = 'center'
        context.textBaseline = 'middle'
        context.fillText(avatarInitial(name), middle, middle + size * 0.03)
    } else {
        context.fillStyle = token('--avatar-image-background')
        context.fillRect(0, 0, size, size)
        context.drawImage(image, 0, 0, size, size)
    }

    // The glass of the list's avatars: a lighter upper half.
    const shine = context.createLinearGradient(0, 0, 0, middle)
    shine.addColorStop(0, token('--avatar-shine-top'))
    shine.addColorStop(1, token('--avatar-shine-bottom'))
    context.fillStyle = shine
    context.fillRect(0, 0, size, middle)
    context.restore()

    context.beginPath()
    context.arc(middle, middle, middle - 1, 0, Math.PI * 2)
    context.strokeStyle = token('--avatar-border')
    context.lineWidth = 2
    context.stroke()

    try {
        return canvas.toDataURL('image/png')
    } catch {
        // A picture the browser will not let out of the canvas.
        return image === null ? null : avatarImage(name, seed, null)
    }
}

// A picture the user chose, cut to a square and made small enough to keep,
// as a PNG `data:` address. Null when the file is not a picture the browser
// can read.
export const iconFromFile = async (file: Blob): Promise<string | null> => {
    let bitmap: ImageBitmap
    try {
        bitmap = await createImageBitmap(file)
    } catch {
        return null
    }

    const drawing = canvasOf(iconPixels)
    if (drawing === null) {
        return null
    }
    const side = Math.min(bitmap.width, bitmap.height)
    const left = (bitmap.width - side) / 2
    const top = (bitmap.height - side) / 2
    drawing.context.drawImage(bitmap, left, top, side, side, 0, 0, iconPixels, iconPixels)
    bitmap.close()

    return drawing.canvas.toDataURL('image/png')
}

import { describe, expect, it } from 'vitest'
import '$lib/design/base.css'
import { avatarImage, iconFromFile, iconPixels } from './avatarImage'

const size = async (url: string): Promise<[number, number]> => {
    const image = new Image()
    image.src = url
    await image.decode()

    return [image.naturalWidth, image.naturalHeight]
}

// A small SVG logo, as sender pictures arrive.
const logo = 'data:image/svg+xml;base64,' + btoa('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="red"/></svg>')

describe('avatarImage', () => {
    it('draws a coloured initial as a PNG', async () => {
        const url = await avatarImage('Ann', 'ann@example.com', null)

        expect(url?.startsWith('data:image/png;base64,')).toBe(true)
    })

    it('draws a sender\'s logo, and the initial when the picture is broken', async () => {
        const drawn = await avatarImage('Ann', 'ann@example.com', logo)
        const initial = await avatarImage('Ann', 'ann@example.com', null)
        const broken = await avatarImage('Ann', 'ann@example.com', 'data:image/svg+xml;base64,AAAA')

        expect(drawn).not.toBe(initial)
        expect(broken).toBe(initial)
    })
})

describe('iconFromFile', () => {
    it('makes a chosen picture a small square PNG', async () => {
        const canvas = document.createElement('canvas')
        canvas.width = 400
        canvas.height = 300
        const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'))
        if (blob === null) {
            throw new Error('no picture to test with')
        }

        const url = await iconFromFile(blob)
        if (url === null) {
            throw new Error('the picture was refused')
        }
        expect(await size(url)).toEqual([iconPixels, iconPixels])
    })

    it('refuses what is not a picture', async () => {
        expect(await iconFromFile(new Blob(['hello'], { type: 'text/plain' }))).toBeNull()
    })
})

import { describe, expect, it } from 'vitest'
import { render } from 'vitest-browser-svelte'
import Avatar from './Avatar.svelte'

// A one-pixel picture that really loads.
const pixel = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw=='

describe('Avatar', () => {
    it('shows the initial when the sender has no picture', async () => {
        const { container } = render(Avatar, { name: 'Jan Gazdik', seed: 'jan@example.com' })

        await expect.element(container).toHaveTextContent('J')
        expect(container.querySelector('img')).toBeNull()
    })

    it('shows the sender\'s own picture when there is one', async () => {
        const { container } = render(Avatar, { name: 'Revolut', seed: 'no-reply@revolut.com', image: pixel })

        await expect.poll(() => container.querySelector('img')?.naturalWidth).toBe(1)
        expect(container.textContent.trim()).toBe('')
    })

    it('falls back to the initial when the picture does not load', async () => {
        const { container } = render(Avatar, {
            name: 'Revolut',
            seed: 'no-reply@revolut.com',
            image: 'data:image/png;base64,broken',
        })

        await expect.element(container).toHaveTextContent('R')
        expect(container.querySelector('img')).toBeNull()
    })
})

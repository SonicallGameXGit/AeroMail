<script lang="ts">
    import type { Snippet } from 'svelte'
    import Button from './Button.svelte'
    import type { IconName } from './icons'

    type Props = {
        // Names the button and the menu.
        label: string
        // The menu's items; they get a function that closes the menu.
        children: Snippet<[() => void]>
        // What the button shows next to its icon. Icon only when left out.
        button?: Snippet
        icon?: IconName
        // A picture of its own in place of `icon`.
        picture?: Snippet
        variant?: 'normal' | 'tool'
        size?: 'normal' | 'small'
        // Which edge of the button the menu lines up with.
        align?: 'start' | 'end'
        disabled?: boolean
    }

    let {
        label,
        children,
        button,
        icon,
        picture,
        variant = 'tool',
        size = 'normal',
        align = 'start',
        disabled = false,
    }: Props = $props()

    const id = $props.id()
    const edge = 8

    let anchor = $state<HTMLElement | null>(null)
    let panel = $state<HTMLElement | null>(null)
    let open = $state(false)
    let top = $state(0)
    let side = $state(0)
    let room = $state(0)

    const close = () => {
        if (open && panel !== null) {
            panel.hidePopover()
        }
    }

    // The menu is drawn above everything (so no pane can clip it), which
    // means it has to be told where its button is.
    const place = (event: ToggleEvent) => {
        if (event.newState !== 'open' || anchor === null) {
            return
        }
        const rect = anchor.getBoundingClientRect()
        top = rect.bottom + 2
        side = align === 'start' ? rect.left : window.innerWidth - rect.right
        room = window.innerHeight - top - edge
    }

    const items = (): HTMLElement[] => {
        if (panel === null) {
            return []
        }

        return Array.from(panel.querySelectorAll('[role^="menuitem"]'))
            .filter((item): item is HTMLElement => item instanceof HTMLElement)
    }

    const step = (event: KeyboardEvent) => {
        if (!open || (event.key !== 'ArrowDown' && event.key !== 'ArrowUp')) {
            return
        }
        const list = items()
        if (list.length === 0) {
            return
        }
        event.preventDefault()

        const current = list.findIndex((item) => item === document.activeElement)
        const move = event.key === 'ArrowDown' ? 1 : -1
        const next = current === -1 && move === -1 ? list.length - 1 : (current + move + list.length) % list.length
        list[next]?.focus()
    }
</script>

<!-- A click inside the message frame never reaches this page; losing focus to it must close the menu too. -->
<svelte:window onblur={close} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="menu" bind:this={anchor} onkeydown={step}>
    {#if button === undefined}
        <Button {variant} {size} {icon} {picture} {label} {disabled} popovertarget={id} expanded={open} />
    {:else}
        <Button {variant} {size} {icon} {picture} {label} {disabled} popovertarget={id} expanded={open}>
            {@render button()}
        </Button>
    {/if}

    <div
        {id}
        class="panel"
        popover="auto"
        role="menu"
        aria-label={label}
        bind:this={panel}
        style:top={top + 'px'}
        style:left={align === 'start' ? side + 'px' : 'auto'}
        style:right={align === 'end' ? side + 'px' : 'auto'}
        style:max-height={room + 'px'}
        onbeforetoggle={place}
        ontoggle={(event) => open = event.newState === 'open'}
    >
        {@render children(close)}
    </div>
</span>

<style>
    .menu {
        display: inline-flex;
        min-width: 0;
    }

    .panel {
        position: fixed;
        bottom: auto;
        margin: 0;
        min-width: 200px;
        max-width: 360px;
        padding: var(--space-1);
        border: 1px solid var(--menu-border);
        border-radius: var(--radius-control);
        background: var(--menu-background);
        color: var(--text);
        box-shadow: var(--menu-shadow);
        overflow-y: auto;
    }

    /* Not on `.panel` itself: that would show the menu while it is closed. */
    .panel:popover-open {
        display: grid;
        gap: 1px;
    }
</style>

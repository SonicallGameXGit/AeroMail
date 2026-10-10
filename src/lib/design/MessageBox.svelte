<script lang="ts">
    import type { Snippet } from 'svelte'
    import MessageIcon from './MessageIcon.svelte'
    import Modal from './Modal.svelte'

    type Props = {
        title: string
        icon: 'success' | 'warning'
        // One short line that says what this is about.
        heading: string
        // The explanation under it.
        children: Snippet
        // The buttons, in the order they are shown; the first one has the focus.
        buttons: Snippet
        // What Escape does, as in a system message box: the safe answer.
        onescape: () => void
    }

    // A short question or announcement with its answers under it, like a
    // system message box: Enter takes the first button, Escape the safe one.
    let { title, icon, heading, children, buttons, onescape }: Props = $props()

    let row = $state<HTMLElement | null>(null)

    $effect(() => {
        if (row === null) {
            return
        }
        const first = row.querySelector('button')
        if (first !== null) {
            first.focus()
        }
    })

    const keydown = (event: KeyboardEvent) => {
        if (event.key === 'Escape') {
            event.preventDefault()
            onescape()
        }
    }
</script>

<svelte:window onkeydown={keydown} />

<Modal {title} size="narrow">
    <div class="message">
        <MessageIcon kind={icon} />
        <div class="text">
            <h2>{heading}</h2>
            <p>{@render children()}</p>
        </div>
    </div>
    <div class="buttons" bind:this={row}>
        {@render buttons()}
    </div>
</Modal>

<style>
    .message {
        display: flex;
        align-items: flex-start;
        gap: var(--space-4);
    }

    .text {
        display: grid;
        gap: var(--space-2);
        min-width: 0;
        /* Starts level with the icon's upper half, not its edge. */
        padding-top: var(--space-1);
        overflow-wrap: anywhere;
    }

    h2 {
        font-size: var(--font-size-heading);
        font-weight: normal;
        color: var(--heading-text);
    }

    /* The grey strip along the bottom of the window, edge to edge: it reaches
       out through the window's content padding. */
    .buttons {
        display: flex;
        justify-content: flex-end;
        gap: var(--space-2);
        margin: var(--space-5) calc(-1 * var(--space-5)) calc(-1 * var(--space-5));
        padding: var(--space-3) var(--space-5);
        border-top: 1px solid var(--toolbar-border);
        background: var(--surface-sunken);
    }

    .buttons :global(button) {
        min-width: var(--dialog-button-width);
        justify-content: center;
    }
</style>

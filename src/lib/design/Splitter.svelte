<script lang="ts">
    // The draggable edge between two panes. It only reports the width the
    // user is asking for; the owner decides what is allowed.
    type Props = {
        label: string
        // Current width of the pane on the left, in pixels.
        value: number
        min: number
        max: number
        onchange: (value: number) => void
        // Double click: back to the default width.
        onreset: () => void
    }

    let { label, value, min, max, onchange, onreset }: Props = $props()

    const keyStep = 16

    // Where the drag started: pointer position and the width at that moment.
    let drag = $state<{ x: number, value: number } | null>(null)

    const start = (event: PointerEvent) => {
        if (event.button !== 0 || !(event.currentTarget instanceof HTMLElement)) {
            return
        }
        // Captured, so the drag keeps working over the message frame.
        event.currentTarget.setPointerCapture(event.pointerId)
        drag = { x: event.clientX, value }
        event.preventDefault()
    }

    const move = (event: PointerEvent) => {
        if (drag !== null) {
            onchange(drag.value + event.clientX - drag.x)
        }
    }

    const stop = () => {
        drag = null
    }

    const keys = (event: KeyboardEvent) => {
        if (event.key === 'ArrowLeft') {
            onchange(value - keyStep)
        } else if (event.key === 'ArrowRight') {
            onchange(value + keyStep)
        } else if (event.key === 'Home') {
            onchange(min)
        } else if (event.key === 'End') {
            onchange(max)
        } else {
            return
        }
        event.preventDefault()
    }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
    class="splitter"
    class:dragging={drag !== null}
    role="separator"
    aria-orientation="vertical"
    aria-label={label}
    aria-valuenow={value}
    aria-valuemin={min}
    aria-valuemax={max}
    title={label}
    tabindex="0"
    onpointerdown={start}
    onpointermove={move}
    onpointerup={stop}
    onpointercancel={stop}
    ondblclick={onreset}
    onkeydown={keys}
></div>

<style>
    .splitter {
        position: relative;
        z-index: 1;
        flex: none;
        width: 1px;
        background: var(--surface-border);
        cursor: col-resize;
        touch-action: none;
    }

    /* The line is one pixel; the area that can be grabbed is wider. */
    .splitter::after {
        content: '';
        position: absolute;
        inset: 0 calc(var(--space-1) * -1);
        transition: background var(--motion-fast);
    }

    .splitter:hover::after,
    .splitter.dragging::after,
    .splitter:focus-visible::after {
        background: var(--splitter-hover);
    }

    .splitter:focus-visible {
        outline: none;
    }
</style>

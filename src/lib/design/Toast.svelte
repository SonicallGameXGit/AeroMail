<script lang="ts">
    import Button from './Button.svelte'

    type Props = {
        // What just happened, in one line.
        text: string
        // The one thing that can be done about it (Undo), if any.
        action?: { label: string, run: () => void }
        closeLabel: string
        onclose: () => void
        // How long it stays when nobody looks at it; the pointer or the
        // keyboard inside it holds it until they leave.
        ms?: number
    }

    let { text, action, closeLabel, onclose, ms = 8000 }: Props = $props()

    let held = $state(false)

    // Starts again from the full time whenever the text changes or it is let go.
    $effect(() => {
        void text
        if (held) {
            return
        }
        const timer = setTimeout(onclose, ms)

        return () => clearTimeout(timer)
    })
</script>

<div
    class="toast"
    role="status"
    onpointerenter={() => held = true}
    onpointerleave={() => held = false}
    onfocusin={() => held = true}
    onfocusout={() => held = false}
>
    <span class="text">{text}</span>
    {#if action !== undefined}
        <Button onclick={action.run}>{action.label}</Button>
    {/if}
    <Button variant="tool" icon="close" label={closeLabel} onclick={onclose} />
</div>

<style>
    .toast {
        position: fixed;
        left: var(--space-4);
        bottom: var(--space-4);
        z-index: 10;
        display: flex;
        align-items: center;
        gap: var(--space-2);
        max-width: min(480px, calc(100vw - 2 * var(--space-4)));
        padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);
        border: 1px solid var(--menu-border);
        border-radius: var(--radius-panel);
        background: var(--toolbar-face);
        color: var(--text);
        box-shadow: var(--menu-shadow), inset 0 1px 0 var(--control-inner);
        animation: rise var(--motion-soft);
    }

    .text {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    @keyframes rise {
        from {
            opacity: 0;
            transform: translateY(8px);
        }
    }
</style>

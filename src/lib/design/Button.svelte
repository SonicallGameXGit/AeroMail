<script lang="ts">
    import type { Snippet } from 'svelte'

    type Props = {
        children: Snippet
        variant?: 'normal' | 'accent' | 'link'
        type?: 'button' | 'submit'
        disabled?: boolean
        onclick?: () => void
    }

    let {
        children,
        variant = 'normal',
        type = 'button',
        disabled = false,
        onclick,
    }: Props = $props()
</script>

<button class={variant} {type} {disabled} {onclick}>
    {@render children()}
</button>

<style>
    button {
        min-height: 26px;
        padding: var(--space-1) var(--space-4);
        border: 1px solid var(--control-border);
        border-radius: var(--radius-control);
        background: var(--control-face);
        color: var(--control-text);
        box-shadow: inset 0 0 0 1px var(--control-inner);
        cursor: default;
        transition: box-shadow var(--motion-fast), border-color var(--motion-fast);
    }

    button:hover:not(:disabled) {
        border-color: var(--control-border-hover);
        background: var(--control-face-hover);
    }

    button:active:not(:disabled) {
        background: var(--control-face-active);
        box-shadow: var(--control-pressed-shadow);
    }

    button:focus-visible {
        outline: none;
        box-shadow: inset 0 0 0 1px var(--control-inner), var(--focus-glow);
    }

    button:disabled {
        opacity: 0.55;
    }

    .accent {
        border-color: var(--accent-border);
        background: var(--accent-face);
        color: var(--accent-text);
        text-shadow: var(--accent-text-shadow);
    }

    .accent:hover:not(:disabled) {
        border-color: var(--accent-border);
        background: var(--accent-face-hover);
    }

    .accent:active:not(:disabled) {
        background: var(--accent-face-active);
    }

    .link,
    .link:hover:not(:disabled),
    .link:active:not(:disabled) {
        padding: var(--space-1);
        border-color: transparent;
        background: none;
        box-shadow: none;
        color: var(--text-link);
    }

    .link:hover:not(:disabled) {
        text-decoration: underline;
    }

    .link:focus-visible {
        box-shadow: var(--focus-glow);
    }
</style>

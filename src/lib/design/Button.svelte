<script lang="ts">
    import type { Snippet } from 'svelte'
    import Icon from './Icon.svelte'
    import type { IconName } from './icons'

    type Props = {
        children?: Snippet
        // `tool` is a toolbar button: flat until the pointer is over it.
        variant?: 'normal' | 'accent' | 'link' | 'tool'
        // `danger` marks an action that destroys something.
        tone?: 'neutral' | 'danger'
        // `small` is a square the size of a field's inner picture, for inside a field.
        size?: 'normal' | 'large' | 'small'
        icon?: IconName
        // A picture of its own in place of `icon`, like a provider's mark.
        picture?: Snippet
        // Names a button that shows only an icon; also its tooltip.
        label?: string
        // Set for a button that stays on or off.
        pressed?: boolean
        type?: 'button' | 'submit'
        disabled?: boolean
        onclick?: () => void
        // Id of the popover this button opens and closes.
        popovertarget?: string
        expanded?: boolean
        // -1 keeps it out of the Tab order, for a control the keyboard reaches another way.
        tabindex?: number
    }

    let {
        children,
        variant = 'normal',
        tone = 'neutral',
        size = 'normal',
        icon,
        picture,
        label,
        pressed,
        type = 'button',
        disabled = false,
        onclick,
        popovertarget,
        expanded,
        tabindex,
    }: Props = $props()

    const iconSizes = {
        normal: 18,
        large: 22,
        small: 14,
    }
</script>

<button
    class={[variant, tone, size, { bare: children === undefined }]}
    {type}
    {disabled}
    {onclick}
    {popovertarget}
    {tabindex}
    title={label}
    aria-label={label}
    aria-pressed={pressed}
    aria-expanded={expanded}
    aria-haspopup={popovertarget === undefined ? undefined : 'menu'}
>
    {#if icon !== undefined}
        <Icon name={icon} size={iconSizes[size]} />
    {:else if picture !== undefined}
        {@render picture()}
    {/if}
    {#if children !== undefined}
        <span class="label">{@render children()}</span>
    {/if}
</button>

<style>
    button {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: var(--space-2);
        min-width: 0;
        min-height: 28px;
        padding: var(--space-1) var(--space-3);
        border: 1px solid var(--control-border);
        border-radius: var(--radius-control);
        background: var(--control-face);
        color: var(--control-text);
        box-shadow: inset 0 0 0 1px var(--control-inner);
        cursor: default;
        transition: box-shadow var(--motion-fast), border-color var(--motion-fast);
    }

    .label {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .bare {
        padding: var(--space-1);
        min-width: 28px;
    }

    .large {
        min-height: 38px;
        padding: var(--space-2) var(--space-4);
        font-weight: 600;
    }

    .large.bare {
        padding: var(--space-2);
    }

    /* No border: what it shows fills it, as it would without the button. */
    .small,
    .small.bare {
        width: var(--field-picture-size);
        height: var(--field-picture-size);
        min-width: 0;
        min-height: 0;
        padding: 0;
        border-width: 0;
    }

    button:hover:not(:disabled) {
        border-color: var(--control-border-hover);
        background: var(--control-face-hover);
    }

    button:active:not(:disabled),
    button[aria-pressed='true']:not(:disabled),
    button[aria-expanded='true']:not(:disabled) {
        background: var(--control-face-active);
        box-shadow: var(--control-pressed-shadow);
    }

    button:focus-visible {
        outline: none;
        box-shadow: inset 0 0 0 1px var(--control-inner), var(--focus-glow);
    }

    button:disabled {
        opacity: 0.45;
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

    .normal.danger {
        border-color: var(--danger-control-border);
        background: var(--danger-face);
        color: var(--danger-control-text);
        text-shadow: var(--danger-text-shadow);
    }

    .normal.danger:hover:not(:disabled) {
        border-color: var(--danger-control-border);
        background: var(--danger-face-hover);
    }

    .normal.danger:active:not(:disabled) {
        background: var(--danger-face-active);
    }

    .tool:not(:hover, :active, :focus-visible, [aria-pressed='true'], [aria-expanded='true']),
    .tool:disabled {
        border-color: transparent;
        background: none;
        box-shadow: none;
        color: inherit;
    }

    .tool.danger:hover:not(:disabled) {
        border-color: var(--danger-control-border);
        background: var(--danger-face);
        color: var(--danger-control-text);
    }

    .tool.danger:active:not(:disabled) {
        background: var(--danger-face-active);
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

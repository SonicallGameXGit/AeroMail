<script lang="ts">
    import type { Snippet } from 'svelte'
    import Icon from './Icon.svelte'
    import type { IconName } from './icons'

    type Props = {
        children: Snippet
        icon?: IconName
        // The item that is already chosen.
        current?: boolean
        indent?: number
        // Set for an item that turns something on and off.
        checked?: boolean
        // A longer explanation, shown as a tooltip.
        hint?: string
        onclick: () => void
    }

    let { children, icon, current = false, indent = 0, checked, hint, onclick }: Props = $props()
</script>

<button
    class="item"
    class:current
    type="button"
    role={checked === undefined ? 'menuitem' : 'menuitemcheckbox'}
    aria-checked={checked}
    aria-current={current}
    title={hint}
    style:padding-left={`calc(var(--space-2) + ${indent} * var(--space-4))`}
    {onclick}
>
    {#if checked !== undefined}
        <span class="tick" class:on={checked}>
            <Icon name="check" size={14} />
        </span>
    {:else if icon !== undefined}
        <Icon name={icon} />
    {/if}
    <span class="text">{@render children()}</span>
</button>

<style>
    .item {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        width: 100%;
        min-width: 0;
        min-height: 28px;
        padding: var(--space-1) var(--space-2);
        border: 1px solid transparent;
        border-radius: var(--radius-control);
        background: none;
        text-align: left;
        cursor: default;
    }

    .item:hover,
    .item:focus-visible {
        border-color: var(--row-hover-border);
        background: var(--row-hover-face);
        outline: none;
    }

    .item.current {
        border-color: var(--row-selected-border);
        background: var(--row-selected-face);
    }

    /* The box is always there; the tick inside it only when the item is on. */
    .tick {
        flex: none;
        display: grid;
        place-items: center;
        width: 18px;
        height: 18px;
        border: 1px solid var(--field-border);
        border-top-color: var(--field-border-top);
        border-radius: var(--radius-control);
        background: var(--field-background);
        box-shadow: var(--field-inner-shadow);
        color: var(--text-link);
    }

    .tick:not(.on) :global(svg) {
        visibility: hidden;
    }

    .text {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
</style>

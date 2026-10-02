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
        onclick: () => void
    }

    let { children, icon, current = false, indent = 0, onclick }: Props = $props()
</script>

<button
    class="item"
    class:current
    type="button"
    role="menuitem"
    aria-current={current}
    style:padding-left={`calc(var(--space-2) + ${indent} * var(--space-4))`}
    {onclick}
>
    {#if icon !== undefined}
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

    .text {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
</style>

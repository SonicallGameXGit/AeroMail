<script lang="ts">
    import Icon from './Icon.svelte'

    type Props = {
        // Names the box; there is no text next to it.
        label: string
        checked: boolean
        // Some of what it stands for is ticked, not all.
        mixed?: boolean
        // Reached by the pointer only, when the keyboard has its own way.
        tabindex?: number
        // Gets the click, so Shift can be read from it.
        onclick: (event: MouseEvent) => void
    }

    let { label, checked, mixed = false, tabindex, onclick }: Props = $props()
</script>

<label class="tick" title={label}>
    <input
        type="checkbox"
        {checked}
        indeterminate={mixed}
        aria-label={label}
        {tabindex}
        onclick={(event) => {
            const box = event.currentTarget
            onclick(event)
            // The box shows what is, not what was clicked.
            box.checked = checked
            box.indeterminate = mixed
        }}
    />
    <span class="box">
        {#if mixed}
            <span class="dash"></span>
        {:else if checked}
            <Icon name="check" size={14} />
        {/if}
    </span>
</label>

<style>
    .tick {
        position: relative;
        flex: none;
        display: grid;
        place-items: center;
        width: 24px;
        height: 24px;
    }

    /* The real checkbox stays for the pointer, the keyboard and screen readers; the box is what is seen. */
    input {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        margin: 0;
        opacity: 0;
    }

    .box {
        display: grid;
        place-items: center;
        width: 16px;
        height: 16px;
        border: 1px solid var(--field-border);
        border-top-color: var(--field-border-top);
        border-radius: var(--radius-control);
        background: var(--field-background);
        box-shadow: var(--field-inner-shadow);
        color: var(--text-link);
        pointer-events: none;
        transition: box-shadow var(--motion-fast), border-color var(--motion-fast);
    }

    .tick:hover .box {
        border-color: var(--control-border-hover);
    }

    input:focus-visible + .box {
        box-shadow: var(--field-inner-shadow), var(--focus-glow);
    }

    .dash {
        width: 8px;
        height: 2px;
        border-radius: 1px;
        background: currentColor;
    }
</style>

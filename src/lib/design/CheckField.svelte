<script lang="ts">
    import Icon from './Icon.svelte'

    type Props = {
        label: string
        checked: boolean
        // A longer explanation of what turning it on or off does.
        hint?: string
        onchange: (checked: boolean) => void
    }

    let { label, checked, hint, onchange }: Props = $props()

    const id = $props.id()
</script>

<label>
    <input
        type="checkbox"
        {checked}
        aria-describedby={hint === undefined ? undefined : id}
        onchange={(event) => {
            const box = event.currentTarget
            onchange(box.checked)
            // The box shows what is, not what was clicked: a change that is refused leaves it as it was.
            box.checked = checked
        }}
    />
    <span class="tick">
        <Icon name="check" size={14} />
    </span>
    <span class="text">
        <span>{label}</span>
        {#if hint !== undefined}
            <small {id}>{hint}</small>
        {/if}
    </span>
</label>

<style>
    label {
        position: relative;
        display: flex;
        align-items: flex-start;
        gap: var(--space-2);
        min-width: 0;
    }

    /* The real checkbox stays for the keyboard and for screen readers; the box next to it is what is seen. */
    input {
        position: absolute;
        width: 18px;
        height: 18px;
        margin: 0;
        opacity: 0;
    }

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
        transition: box-shadow var(--motion-fast);
    }

    input:not(:checked) + .tick :global(svg) {
        visibility: hidden;
    }

    input:focus-visible + .tick {
        box-shadow: var(--field-inner-shadow), var(--focus-glow);
    }

    .text {
        display: grid;
        gap: var(--space-1);
        min-width: 0;
    }

    small {
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }
</style>

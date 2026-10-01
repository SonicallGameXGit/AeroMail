<script lang="ts" generics="Value extends string">
    type Props = {
        label: string
        value: Value
        options: { value: Value, label: string }[]
        onchange: (value: Value) => void
    }

    let { label, value, options, onchange }: Props = $props()

    const select = (raw: string) => {
        const option = options.find((candidate) => candidate.value === raw)
        if (option !== undefined) {
            onchange(option.value)
        }
    }
</script>

<label>
    <span>{label}</span>
    <select {value} onchange={(event) => select(event.currentTarget.value)}>
        {#each options as option (option.value)}
            <option value={option.value}>{option.label}</option>
        {/each}
    </select>
</label>

<style>
    label {
        display: grid;
        gap: var(--space-1);
        min-width: 0;
    }

    span {
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    select {
        width: 100%;
        min-height: 26px;
        padding: var(--space-1) var(--space-2);
        border: 1px solid var(--control-border);
        border-radius: var(--radius-control);
        background: var(--control-face);
        color: var(--control-text);
        box-shadow: inset 0 0 0 1px var(--control-inner);
    }

    select:hover {
        border-color: var(--control-border-hover);
        background: var(--control-face-hover);
    }

    select:focus-visible {
        outline: none;
        box-shadow: inset 0 0 0 1px var(--control-inner), var(--focus-glow);
    }

    option {
        background: var(--field-background);
        color: var(--text);
    }
</style>

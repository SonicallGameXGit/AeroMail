<script lang="ts">
    type Props = {
        label: string
        value: string
        type?: 'text' | 'email' | 'password'
        inputmode?: 'text' | 'email' | 'numeric'
        placeholder?: string
        autofocus?: boolean
        invalid?: boolean
        oninput?: (value: string) => void
    }

    let {
        label,
        value,
        type = 'text',
        inputmode = 'text',
        placeholder = '',
        autofocus = false,
        invalid = false,
        oninput,
    }: Props = $props()

    const focusOnMount = (node: HTMLInputElement) => {
        if (autofocus) {
            node.focus()
        }
    }
</script>

<label>
    <span>{label}</span>
    <input
        {type}
        {inputmode}
        {placeholder}
        {value}
        aria-invalid={invalid}
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        use:focusOnMount
        oninput={(event) => oninput?.(event.currentTarget.value)}
    />
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

    input {
        width: 100%;
        min-height: 26px;
        padding: var(--space-1) var(--space-2);
        border: 1px solid var(--field-border);
        border-top-color: var(--field-border-top);
        border-radius: var(--radius-control);
        background: var(--field-background);
        box-shadow: var(--field-inner-shadow);
        transition: box-shadow var(--motion-fast);
    }

    input:focus-visible {
        outline: none;
        box-shadow: var(--field-inner-shadow), var(--focus-glow);
    }

    input[aria-invalid='true'] {
        border-color: var(--danger-border);
    }
</style>

<script lang="ts">
    type Props = {
        label: string
        value: string
        type?: 'text' | 'email' | 'password'
        inputmode?: 'text' | 'email' | 'numeric'
        placeholder?: string
        autofocus?: boolean
        invalid?: boolean
        // A box for several lines of text.
        multiline?: boolean
        // A longer explanation under the field.
        hint?: string
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
        multiline = false,
        hint,
        oninput,
    }: Props = $props()

    const id = $props.id()

    const focusOnMount = (node: HTMLInputElement | HTMLTextAreaElement) => {
        if (autofocus) {
            node.focus()
        }
    }
</script>

<label>
    <span>{label}</span>
    {#if multiline}
        <textarea
            rows="3"
            {placeholder}
            {value}
            aria-invalid={invalid}
            aria-describedby={hint === undefined ? undefined : id}
            autocomplete="off"
            spellcheck="false"
            use:focusOnMount
            oninput={(event) => oninput?.(event.currentTarget.value)}
        ></textarea>
    {:else}
        <input
            {type}
            {inputmode}
            {placeholder}
            {value}
            aria-invalid={invalid}
            aria-describedby={hint === undefined ? undefined : id}
            autocomplete="off"
            autocapitalize="off"
            spellcheck="false"
            use:focusOnMount
            oninput={(event) => oninput?.(event.currentTarget.value)}
        />
    {/if}
    {#if hint !== undefined}
        <small {id}>{hint}</small>
    {/if}
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

    input,
    textarea {
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

    textarea {
        resize: vertical;
        font: inherit;
    }

    small {
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    input:focus-visible,
    textarea:focus-visible {
        outline: none;
        box-shadow: var(--field-inner-shadow), var(--focus-glow);
    }

    input[aria-invalid='true'],
    textarea[aria-invalid='true'] {
        border-color: var(--danger-border);
    }
</style>

<script lang="ts">
    import type { Snippet } from 'svelte'

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
        // Something small shown inside a one-line field, at its end.
        trailing?: Snippet
        // Fixed text after what is typed, shown dimmed and not editable (`@gmail.com`).
        suffix?: string
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
        trailing,
        suffix,
        oninput,
    }: Props = $props()

    const id = $props.id()

    // What is typed must stop before the suffix, whatever its length.
    let suffixWidth = $state(0)
    // Read with the field, so the fixed part is heard too.
    const described = $derived([
        hint === undefined ? null : id,
        suffix === undefined ? null : id + '-suffix',
    ].filter((part) => part !== null).join(' '))
    const room = $derived(trailing === undefined ? 'var(--space-2)' : 'var(--field-trailing-room)')

    const focusOnMount = (node: HTMLInputElement | HTMLTextAreaElement) => {
        if (autofocus) {
            // After the frame: a menu that opened this window hands the focus
            // back to its button when it closes, which would undo ours.
            requestAnimationFrame(() => node.focus())
        }
    }
</script>

<label>
    <span class="caption">{label}</span>
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
        <span class="control">
            <input
                {type}
                {inputmode}
                {placeholder}
                {value}
                class:roomy={trailing !== undefined}
                style:padding-right={suffix === undefined ? undefined : `calc(${room} + ${suffixWidth}px)`}
                aria-invalid={invalid}
                aria-describedby={described === '' ? undefined : described}
                autocomplete="off"
                autocapitalize="off"
                spellcheck="false"
                use:focusOnMount
                oninput={(event) => oninput?.(event.currentTarget.value)}
            />
            {#if suffix !== undefined}
                <span id={id + '-suffix'} class="suffix" class:roomy={trailing !== undefined} bind:clientWidth={suffixWidth}>{suffix}</span>
            {/if}
            {#if trailing !== undefined}
                <span class="trailing">{@render trailing()}</span>
            {/if}
        </span>
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

    .caption {
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }

    .control {
        position: relative;
        display: block;
    }

    .trailing {
        position: absolute;
        top: 0;
        right: var(--space-1);
        bottom: 0;
        display: flex;
        align-items: center;
    }

    /* Clicks go through to the field, so the text is typed as if it ended there. */
    .suffix {
        position: absolute;
        top: 0;
        right: var(--space-2);
        bottom: 0;
        display: flex;
        align-items: center;
        color: var(--text-muted);
        white-space: nowrap;
        pointer-events: none;
        user-select: none;
        -webkit-user-select: none;
    }

    .suffix.roomy {
        right: var(--field-trailing-room);
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

    .roomy {
        padding-right: var(--field-trailing-room);
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

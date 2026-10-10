<script lang="ts">
    import type { Theme } from '$lib/api'
    import ThemePreview from '$lib/design/ThemePreview.svelte'
    import { themes } from '$lib/design/theme'
    import { t } from '$lib/i18n'

    type Props = {
        theme: Theme
        onchange: (theme: Theme) => void
    }

    let { theme, onchange }: Props = $props()

    const choices: Record<string, HTMLButtonElement> = {}

    // Arrow keys move the choice, as in any group of radio buttons; Home and End jump to its ends.
    const walk = (event: KeyboardEvent, index: number) => {
        const last = themes.length - 1
        const next = index === last ? 0 : index + 1
        const previous = index === 0 ? last : index - 1
        const moves: Record<string, number> = {
            ArrowRight: next,
            ArrowDown: next,
            ArrowLeft: previous,
            ArrowUp: previous,
            Home: 0,
            End: last,
        }
        const target = themes[moves[event.key] ?? -1]
        if (target === undefined) {
            return
        }

        event.preventDefault()
        onchange(target)
        choices[target]?.focus()
    }
</script>

<div class="themes" role="radiogroup" aria-label={t('settings.theme')}>
    {#each themes as option, index (option)}
        <!-- Checked is what the settings say, not what was clicked: a change that is refused shows nothing. -->
        <button
            bind:this={choices[option]}
            class="choice"
            class:selected={option === theme}
            type="button"
            role="radio"
            aria-checked={option === theme}
            tabindex={option === theme ? 0 : -1}
            onclick={() => onchange(option)}
            onkeydown={(event) => walk(event, index)}
        >
            <ThemePreview theme={option} />
            <span class="name">{t(`theme.${option}`)}</span>
            <small>{t(`theme.${option}Hint`)}</small>
        </button>
    {/each}
</div>

<style>
    /* Side by side while they fit, like the choices of a system's own personalisation. */
    .themes {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(112px, 1fr));
        gap: var(--space-2);
    }

    .choice {
        display: grid;
        justify-items: center;
        gap: var(--space-1);
        padding: var(--space-2);
        border: 1px solid transparent;
        border-radius: var(--radius-panel);
        background: none;
        color: var(--text);
        cursor: default;
    }

    .choice:hover {
        border-color: var(--row-hover-border);
        background: var(--row-hover-face);
    }

    .choice.selected {
        border-color: var(--row-selected-border);
        background: var(--row-selected-face);
    }

    .choice:focus-visible {
        outline: none;
        box-shadow: var(--focus-glow);
    }

    .name {
        font-weight: 600;
    }

    small {
        font-size: var(--font-size-small);
        color: var(--text-muted);
    }
</style>

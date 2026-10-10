<script lang="ts">
    import type { Theme } from '$lib/api'
    import { themeAttribute } from './theme'

    // A miniature of the app in a theme, drawn with that theme's own tokens
    // (tokens.css sets them on any element with a `data-theme`). A theme that
    // follows the system shows the light and the dark one, half each.
    let { theme }: { theme: Theme } = $props()

    let attribute = $derived(themeAttribute(theme))
</script>

{#snippet scene(attribute: string)}
    <div class="scene" data-theme={attribute}>
        <div class="glass">
            <div class="pane">
                <div class="row selected"></div>
                <div class="line"></div>
                <div class="line short"></div>
                <div class="button"></div>
            </div>
        </div>
    </div>
{/snippet}

<div class="preview" aria-hidden="true">
    {#if attribute === null}
        {@render scene('aero')}
        <div class="half">
            {@render scene('slate')}
        </div>
    {:else}
        {@render scene(attribute)}
    {/if}
</div>

<style>
    .preview {
        position: relative;
        width: 100%;
        aspect-ratio: 11 / 7;
        overflow: hidden;
        border: 1px solid var(--surface-border);
        border-radius: var(--radius-control);
    }

    .half {
        position: absolute;
        inset: 0;
        clip-path: inset(0 0 0 50%);
    }

    .scene {
        position: absolute;
        inset: 0;
        padding: 10px 12px 0;
        background:
            radial-gradient(ellipse 90% 55% at 50% -10%, var(--backdrop-light), transparent 70%),
            linear-gradient(180deg, var(--backdrop-top) 0%, var(--backdrop-middle) 55%, var(--backdrop-bottom) 100%);
    }

    .glass {
        height: 100%;
        padding: 9px 4px 0;
        border: 1px solid var(--glass-border-outer);
        border-bottom: none;
        border-radius: var(--radius-control) var(--radius-control) 0 0;
        background: var(--glass-shine), var(--glass-tint);
        box-shadow: inset 0 0 0 1px var(--glass-border-inner);
    }

    .pane {
        display: grid;
        align-content: start;
        gap: 5px;
        height: 100%;
        padding: 5px;
        border: 1px solid var(--surface-border);
        border-bottom: none;
        background: var(--surface);
    }

    .row {
        height: 9px;
        border: 1px solid var(--row-selected-border);
        border-radius: 2px;
        background: var(--row-selected-face);
    }

    .line {
        width: 85%;
        height: 3px;
        border-radius: 2px;
        background: var(--text-muted);
    }

    .short {
        width: 55%;
    }

    .button {
        justify-self: end;
        width: 28px;
        height: 9px;
        border: 1px solid var(--accent-border);
        border-radius: 2px;
        background: var(--accent-face);
    }
</style>

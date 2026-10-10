<script lang="ts">
    import type { Snippet } from 'svelte'

    // Frutiger Aero imagery: only behind glass chrome, never under mail content.
    // `fill` lets the content take the whole window instead of floating in the centre.
    let { children, fill = false }: { children: Snippet, fill?: boolean } = $props()
</script>

<div class="backdrop" class:fill>
    <div class="bokeh" aria-hidden="true"></div>
    {@render children()}
</div>

<style>
    .backdrop {
        position: fixed;
        inset: 0;
        display: grid;
        place-items: center;
        padding: var(--space-5);
        overflow: auto;
        background:
            radial-gradient(ellipse 90% 55% at 50% -10%, var(--backdrop-light), transparent 70%),
            linear-gradient(180deg, var(--backdrop-top) 0%, var(--backdrop-middle) 55%, var(--backdrop-bottom) 100%);
    }

    .fill {
        place-items: stretch;
        padding: 0;
        overflow: hidden;
    }

    .bokeh {
        position: absolute;
        inset: 0;
        pointer-events: none;
        background:
            radial-gradient(circle at 14% 72%, var(--backdrop-bokeh) 0 70px, transparent 72px),
            radial-gradient(circle at 24% 86%, var(--backdrop-bokeh) 0 38px, transparent 40px),
            radial-gradient(circle at 82% 68%, var(--backdrop-bokeh) 0 96px, transparent 98px),
            radial-gradient(circle at 70% 88%, var(--backdrop-bokeh) 0 46px, transparent 48px),
            radial-gradient(circle at 90% 22%, var(--backdrop-bokeh) 0 30px, transparent 32px),
            radial-gradient(circle at 8% 24%, var(--backdrop-bokeh) 0 52px, transparent 54px);
    }
</style>

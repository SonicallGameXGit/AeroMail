<script lang="ts">
    import type { Snippet } from 'svelte'

    // An Aero window: glass frame with a title, solid content inside.
    let { title, children }: { title: string, children: Snippet } = $props()
</script>

<section class="window" aria-label={title}>
    <header class="title">{title}</header>
    <div class="content">
        {@render children()}
    </div>
</section>

<style>
    .window {
        position: relative;
        width: min(560px, 100%);
        padding: 0 var(--space-2) var(--space-2);
        border: 1px solid var(--glass-border-outer);
        border-radius: var(--radius-window);
        background: var(--glass-shine), var(--glass-tint);
        backdrop-filter: blur(var(--glass-blur));
        -webkit-backdrop-filter: blur(var(--glass-blur));
        box-shadow: inset 0 0 0 1px var(--glass-border-inner), var(--glass-shadow);
    }

    .title {
        padding: var(--space-2) var(--space-1);
        color: var(--glass-text);
        text-shadow: var(--glass-text-shadow);
    }

    .content {
        padding: var(--space-5);
        border: 1px solid var(--surface-border);
        border-radius: var(--radius-control);
        background: var(--surface);
        color: var(--text);
    }
</style>

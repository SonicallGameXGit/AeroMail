<script lang="ts">
    import type { Snippet } from 'svelte'

    // An Aero window: glass frame with a title, solid content inside.
    type Props = {
        title: string
        children: Snippet
        // narrow: a message box, as wide as its text needs.
        size?: 'narrow' | 'normal' | 'wide'
    }

    let { title, children, size = 'normal' }: Props = $props()
</script>

<section class="window" class:wide={size === 'wide'} class:narrow={size === 'narrow'} aria-label={title}>
    <header class="title">{title}</header>
    <div class="content">
        {@render children()}
    </div>
</section>

<style>
    .window {
        position: relative;
        width: min(560px, 100%);
        max-height: 100%;
        display: flex;
        flex-direction: column;
        padding: 0 var(--space-2) var(--space-2);
        border: 1px solid var(--glass-border-outer);
        border-radius: var(--radius-window);
        background: var(--glass-shine), var(--glass-tint);
        backdrop-filter: blur(var(--glass-blur));
        -webkit-backdrop-filter: blur(var(--glass-blur));
        box-shadow: inset 0 0 0 1px var(--glass-border-inner), var(--glass-shadow);
    }

    .wide {
        width: min(720px, 100%);
    }

    .narrow {
        width: fit-content;
        min-width: min(360px, 100%);
        max-width: min(480px, 100%);
    }

    .title {
        padding: var(--space-2) var(--space-1);
        color: var(--glass-text);
        text-shadow: var(--glass-text-shadow);
    }

    .content {
        min-height: 0;
        overflow-y: auto;
        padding: var(--space-5);
        border: 1px solid var(--surface-border);
        border-radius: var(--radius-control);
        background: var(--surface);
        color: var(--text);
    }
</style>

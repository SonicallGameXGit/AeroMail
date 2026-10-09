<script lang="ts">
    import type { Snippet } from 'svelte'
    import GlassWindow from './GlassWindow.svelte'

    type Props = {
        title: string
        children: Snippet
        wide?: boolean
    }

    // A glass window above the workspace; everything behind it is dimmed.
    let { title, children, wide = false }: Props = $props()
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-label={title}>
    <GlassWindow {title} {wide}>
        {@render children()}
    </GlassWindow>
</div>

<style>
    .overlay {
        position: fixed;
        inset: 0;
        z-index: 10;
        display: grid;
        /* One cell the size of the screen: a window taller than that scrolls
           inside instead of running off the bottom. */
        grid-template: minmax(0, 1fr) / minmax(0, 1fr);
        place-items: center;
        padding: var(--space-5);
        background: var(--overlay);
    }
</style>

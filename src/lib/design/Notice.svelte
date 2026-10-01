<script lang="ts">
    import type { Snippet } from 'svelte'

    type Props = {
        children: Snippet
        kind?: 'info' | 'success' | 'warning' | 'danger'
    }

    let { children, kind = 'info' }: Props = $props()
</script>

<div class={kind} role={kind === 'danger' ? 'alert' : 'status'}>
    {@render children()}
</div>

<style>
    div {
        padding: var(--space-2) var(--space-3);
        border: 1px solid var(--info-border);
        border-radius: var(--radius-control);
        background: var(--info-background);
        user-select: text;
        -webkit-user-select: text;
    }

    .success {
        border-color: var(--success-border);
        background: var(--success-background);
    }

    .warning {
        border-color: var(--warning-border);
        background: var(--warning-background);
    }

    .danger {
        border-color: var(--danger-border);
        background: var(--danger-background);
    }
</style>

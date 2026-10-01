<script lang="ts">
    import '$lib/design/base.css'
    import favicon from '$lib/assets/favicon.svg'
    import { t } from '$lib/i18n'

    let { children } = $props()
</script>

<svelte:head>
    <title>{t('app.name')}</title>
    <link rel="icon" href={favicon} />
</svelte:head>

{@render children()}

<script lang="ts">
    import { tauriApi } from '$lib/api'
    import Backdrop from '$lib/design/Backdrop.svelte'
    import GlassWindow from '$lib/design/GlassWindow.svelte'
    import AccountSetup from '$lib/features/accounts/AccountSetup.svelte'
    import { t } from '$lib/i18n'
</script>

<Backdrop>
    <GlassWindow title={t('app.name')}>
        <AccountSetup api={tauriApi} />
    </GlassWindow>
</Backdrop>

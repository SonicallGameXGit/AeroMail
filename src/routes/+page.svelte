<script lang="ts">
    import { tauriApi } from '$lib/api'
    import Workspace from '$lib/features/workspace/Workspace.svelte'
</script>

<Workspace api={tauriApi} />

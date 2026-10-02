<script lang="ts">
    import { desktopApi } from '$lib/api'
    import Workspace from '$lib/features/workspace/Workspace.svelte'
</script>

<Workspace api={desktopApi} />

<script lang="ts">
    import { avatarInitial, avatarTone } from './avatar'

    type Props = {
        // What the initial is taken from.
        name: string
        // What the colour is taken from: the address, which never changes.
        seed: string
        // The sender's own picture, when there is one.
        image?: string | null
        size?: 'small' | 'normal' | 'large'
    }

    let { name, seed, image = null, size = 'normal' }: Props = $props()

    // A picture that would not load; the initial stands in for it.
    let broken = $state<string | null>(null)
    let picture = $derived(image !== null && image !== broken ? image : null)
</script>

<span class={['avatar', size]} style:--tone={`var(--avatar-${avatarTone(seed)})`} aria-hidden="true">
    {#if picture !== null}
        <img src={picture} alt="" draggable="false" onerror={() => broken = picture} />
    {:else}
        {avatarInitial(name)}
    {/if}
</span>

<style>
    .avatar {
        position: relative;
        flex: none;
        display: inline-grid;
        place-items: center;
        width: 30px;
        height: 30px;
        border: 1px solid var(--avatar-border);
        border-radius: 50%;
        background: var(--tone);
        color: var(--avatar-text);
        font-weight: 600;
        line-height: 1;
        text-shadow: var(--avatar-text-shadow);
        overflow: hidden;
    }

    img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        /* Logos are often drawn on nothing; they were made for a white page. */
        background: var(--avatar-image-background);
    }

    /* The glass sits above whatever is inside, so a sender's own picture
       and a plain initial belong to the same set. */
    .avatar::after {
        content: '';
        position: absolute;
        inset: 0;
        border-radius: 50%;
        background: var(--avatar-shine);
        box-shadow: var(--avatar-glass);
        pointer-events: none;
    }

    .small {
        width: 22px;
        height: 22px;
        font-size: var(--font-size-small);
    }

    .large {
        width: 38px;
        height: 38px;
        font-size: var(--font-size-title);
        font-weight: 400;
    }
</style>

<script lang="ts">
    import { avatarInitial, avatarTone } from './avatar'

    type Props = {
        // What the initial is taken from.
        name: string
        // What the colour is taken from: the address, which never changes.
        seed: string
        size?: 'small' | 'normal' | 'large'
    }

    let { name, seed, size = 'normal' }: Props = $props()
</script>

<span class={['avatar', size]} style:--tone={`var(--avatar-${avatarTone(seed)})`} aria-hidden="true">
    {avatarInitial(name)}
</span>

<style>
    .avatar {
        flex: none;
        display: inline-grid;
        place-items: center;
        width: 30px;
        height: 30px;
        border: 1px solid var(--avatar-border);
        border-radius: 50%;
        background: var(--avatar-shine), var(--tone);
        color: var(--avatar-text);
        font-weight: 600;
        line-height: 1;
        text-shadow: var(--avatar-text-shadow);
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

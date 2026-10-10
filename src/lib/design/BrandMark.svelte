<script lang="ts">
    import type { Brand } from '$lib/api'

    // A provider's own mark on a small tile, the way an app's icon sat on
    // Windows 7: every colour of the logo is a glossy material (lighter at the
    // top), and the tile is under the same glass as the avatars, so a foreign
    // logo looks part of the theme without being redrawn in other colours.
    type Props = {
        brand: Brand
        // The tooltip; the mark itself is hidden from screen readers.
        label: string
        size?: number
    }

    let { brand, label, size = 20 }: Props = $props()

    // Gradients are referenced by id, and ids must not clash between marks.
    const uid = $props.id()
    const url = (material: string) => `url(#${uid}-${material})`

    const materials = [
        'blue',
        'green',
        'yellow',
        'red',
        'deep-red',
    ]
</script>

<span class="tile" style:width={size + 'px'} style:height={size + 'px'} title={label} aria-hidden="true">
    {#if brand === 'Gmail'}
        <svg class="mark" viewBox="52 42 88 66" focusable="false">
            <defs>
                {#each materials as material (material)}
                    <linearGradient id={`${uid}-${material}`} x1="0" y1="0" x2="0" y2="1">
                        <stop offset="0" style:stop-color={`var(--brand-google-${material}-light)`} />
                        <stop offset="1" style:stop-color={`var(--brand-google-${material}-dark)`} />
                    </linearGradient>
                {/each}
            </defs>
            <path fill={url('blue')} d="M58 108h14V74L52 59v43c0 3.32 2.69 6 6 6" />
            <path fill={url('green')} d="M120 108h14c3.32 0 6-2.69 6-6V59l-20 15" />
            <path fill={url('yellow')} d="M120 48v26l20-15v-8c0-7.42-8.47-11.65-14.4-7.2" />
            <path fill={url('red')} d="M72 74V48l24 18 24-18v26L96 92" />
            <path fill={url('deep-red')} d="M52 51v8l20 15V48l-5.6-4.2c-5.94-4.45-14.4-.22-14.4 7.2" />
        </svg>
    {/if}
</span>

<style>
    .tile {
        position: relative;
        flex: none;
        display: inline-grid;
        place-items: center;
        border: 1px solid var(--brand-tile-border);
        border-radius: var(--radius-control);
        background: linear-gradient(180deg, var(--icon-paper-light), var(--icon-paper-dark));
        box-shadow: var(--brand-tile-shadow);
        overflow: hidden;
        animation: appear var(--motion-fast);
    }

    .mark {
        display: block;
        width: 74%;
        filter: drop-shadow(var(--brand-mark-shadow));
    }

    /* The glass sits above the logo, as over an avatar's picture. */
    .tile::after {
        content: '';
        position: absolute;
        inset: 0;
        border-radius: inherit;
        background: var(--avatar-shine);
        box-shadow: var(--avatar-glass);
        pointer-events: none;
    }

    @keyframes appear {
        from {
            opacity: 0;
        }
    }
</style>

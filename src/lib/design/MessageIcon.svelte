<script lang="ts">
    // The big picture of a message box, in the old system style: a glossy
    // sphere or sign with a rounded highlight on top, lit from above, standing
    // on its own soft shadow. Drawn on a 48 grid.
    let { kind, size = 40 }: { kind: 'success' | 'warning', size?: number } = $props()

    // Gradients are referenced by id, and ids must not clash between icons.
    const uid = $props.id()
    const id = (name: string) => `${uid}-${name}`
    const url = (name: string) => `url(#${id(name)})`

    const triangle = 'M24 4.5c1.3 0 2.4.7 3.1 1.9l17 30.2c1.4 2.5-.4 5.4-3.2 5.4H7.1c-2.8 0-4.6-2.9-3.2-5.4l17-30.2c.7-1.2 1.8-1.9 3.1-1.9z'
</script>

<svg
    class="icon"
    width={size}
    height={size}
    viewBox="0 0 48 48"
    aria-hidden="true"
    focusable="false"
    stroke-linejoin="round"
    stroke-linecap="round"
>
    <defs>
        <radialGradient id={id('shadow')}>
            <stop offset="0" style:stop-color="var(--icon-shadow)" />
            <stop offset="1" style:stop-color="var(--icon-shadow)" stop-opacity="0" />
        </radialGradient>
        <linearGradient id={id('gloss')} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" style:stop-color="var(--icon-gloss-top)" />
            <stop offset="1" style:stop-color="var(--icon-gloss-bottom)" />
        </linearGradient>
        <radialGradient id={id('green')} cx="0.42" cy="0.32" r="0.75">
            <stop offset="0" style:stop-color="var(--icon-green-light)" />
            <stop offset="0.55" style:stop-color="var(--icon-green-dark)" />
            <stop offset="1" style:stop-color="var(--icon-green-edge)" />
        </radialGradient>
        <!-- Light thrown back from below, as on a glass marble. -->
        <radialGradient id={id('green-glow')} cx="0.5" cy="1" r="0.6">
            <stop offset="0" style:stop-color="var(--icon-green-light)" stop-opacity="0.8" />
            <stop offset="1" style:stop-color="var(--icon-green-light)" stop-opacity="0" />
        </radialGradient>
        <linearGradient id={id('gold')} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" style:stop-color="var(--icon-gold-light)" />
            <stop offset="1" style:stop-color="var(--icon-gold-dark)" />
        </linearGradient>
    </defs>

    <ellipse cx="24" cy="44" rx="16" ry="3.5" fill={url('shadow')} />

    {#if kind === 'success'}
        <circle class="green" cx="24" cy="22" r="19" fill={url('green')} />
        <circle cx="24" cy="22" r="18.5" fill={url('green-glow')} />
        <ellipse cx="24" cy="12.5" rx="13.5" ry="8.5" fill={url('gloss')} />
        <path class="ink-shadow" d="M15 22.5l6.5 6.5L33.5 15.5" transform="translate(0 1.2)" />
        <path class="ink" d="M15 22.5l6.5 6.5L33.5 15.5" />
    {:else if kind === 'warning'}
        <path class="gold" d={triangle} fill={url('gold')} />
        <path d="M24 6.8c.6 0 1.1.3 1.4.9L36 26.5c-7-2.5-17-2.5-24 0L22.6 7.7c.3-.6.8-.9 1.4-.9z" fill={url('gloss')} />
        <path class="mark" d="M24 15.5v10.5" />
        <circle class="mark-dot" cx="24" cy="33" r="2.6" />
    {/if}
</svg>

<style>
    .icon {
        flex: none;
        display: block;
    }

    .green {
        stroke: var(--icon-green-edge);
        stroke-width: 1;
    }

    .gold {
        stroke: var(--icon-gold-edge);
        stroke-width: 1;
    }

    .ink {
        fill: none;
        stroke: var(--icon-ink);
        stroke-width: 5;
    }

    .ink-shadow {
        fill: none;
        stroke: var(--icon-shadow);
        stroke-width: 5.5;
    }

    .mark {
        fill: none;
        stroke: var(--icon-ink-dark);
        stroke-width: 4.5;
    }

    .mark-dot {
        fill: var(--icon-ink-dark);
    }
</style>

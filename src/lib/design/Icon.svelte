<script lang="ts">
    import type { IconName } from './icons'

    // Aero icons: each shape is a glossy material (light at the top, darker
    // below) with a darker edge. Drawn on a 24 grid. Only the small structural
    // glyphs (chevron, close, check) are flat and take the text colour.
    let { name, size = 18 }: { name: IconName, size?: number } = $props()

    // Gradients are referenced by id, and ids must not clash between icons.
    const uid = $props.id()
    const blue = `url(#${uid}-blue)`
    const green = `url(#${uid}-green)`
    const gold = `url(#${uid}-gold)`
    const red = `url(#${uid}-red)`
    const paper = `url(#${uid}-paper)`
    const steel = `url(#${uid}-steel)`

    const materials = [
        'blue',
        'green',
        'gold',
        'red',
        'paper',
        'steel',
    ]

    const replyArrow = 'M10 4.5 3 11l7 6.5v-4c5 0 8.5 1.5 10.5 5 0-6-3.5-10-10.5-10z'
    const folderShape = 'M2.5 7A1.5 1.5 0 0 1 4 5.5h5l2 2h9A1.5 1.5 0 0 1 21.5 9v9.5A1.5 1.5 0 0 1 20 20H4a1.5 1.5 0 0 1-1.5-1.5z'
    const flagCloth = 'M6.5 4.5c3-1.5 5 1.5 8 .5 1.5-.5 2.5-1 4.5-1v9c-2 0-3 .5-4.5 1-3 1-5-2-8-.5z'
</script>

<svg
    class="icon"
    width={size}
    height={size}
    viewBox="0 0 24 24"
    aria-hidden="true"
    focusable="false"
    stroke-linejoin="round"
    stroke-linecap="round"
>
    <defs>
        {#each materials as material (material)}
            <linearGradient id={`${uid}-${material}`} x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" style:stop-color={`var(--icon-${material}-light)`} />
                <stop offset="1" style:stop-color={`var(--icon-${material}-dark)`} />
            </linearGradient>
        {/each}
    </defs>

    {#if name === 'chevron'}
        <path class="glyph" d="M7 9.5l5 5 5-5" />
    {:else if name === 'close'}
        <path class="glyph" d="M6.5 6.5l11 11M17.5 6.5l-11 11" />
    {:else if name === 'check'}
        <path class="glyph" d="M5 12.5l4.5 4.5L19 7.5" />
    {:else if name === 'compose'}
        <rect class="paper" x="3.5" y="5.5" width="14" height="16" rx="1.5" fill={paper} />
        <path class="rule" d="M6.5 10h5M6.5 13.5h4M6.5 17h8" />
        <path class="gold" d="M19 2.6l2.6 2.6L12 14.8l-3.6 1 1-3.6z" fill={gold} />
        <path class="gold" d="M17.2 4.4l2.6 2.6" fill="none" />
    {:else if name === 'reply'}
        <path class="blue" d={replyArrow} fill={blue} />
    {:else if name === 'replyAll'}
        <path class="blue" d="M7.5 4.5 1 11l6.5 6.5z" fill={blue} />
        <path class="blue" d="M13.5 4.5 6.5 11l7 6.5v-4c4 0 7 1.5 9 5 0-6-3-10-9-10z" fill={blue} />
    {:else if name === 'forward'}
        <path class="green" d={replyArrow} fill={green} transform="matrix(-1 0 0 1 24 0)" />
    {:else if name === 'archive'}
        <path class="gold" d="M4 9.5h16v9a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 18.5z" fill={gold} />
        <rect class="gold" x="2.5" y="4.5" width="19" height="5" rx="1" fill={gold} />
        <rect class="paper" x="9" y="12.5" width="6" height="2.5" rx="1.25" fill={paper} />
    {:else if name === 'trash'}
        <path class="steel" d="M9.5 5.5v-1a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1v1" fill="none" />
        <path class="steel" d="M6 8.5h12l-1 11a1.5 1.5 0 0 1-1.5 1.4h-7A1.5 1.5 0 0 1 7 19.5z" fill={steel} />
        <rect class="steel" x="4.5" y="5.5" width="15" height="3" rx="1" fill={steel} />
        <path class="groove" d="M10 11.5v6.5M14 11.5v6.5" />
    {:else if name === 'folder'}
        <path class="gold" d={folderShape} fill={gold} />
        <path class="shine" d="M3.5 10.5h17v1.5h-17z" />
    {:else if name === 'move'}
        <path class="gold" d={folderShape} fill={gold} />
        <path class="blue" d="M10.5 12.5h4.5V10l5 4.2-5 4.3V16h-4.5z" fill={blue} />
    {:else if name === 'mail' || name === 'mailUnread'}
        <rect class="paper" x="2.5" y="5.5" width="19" height="13.5" rx="1.5" fill={paper} />
        <path class="paper" d="M3.2 6.5l8.8 7 8.8-7" fill="none" />
        {#if name === 'mailUnread'}
            <circle class="blue" cx="18.5" cy="6.5" r="3.6" fill={blue} />
        {/if}
    {:else if name === 'mailOpen'}
        <path class="paper" d="M2.5 9.5 12 3l9.5 6.5v10A1.5 1.5 0 0 1 20 21H4a1.5 1.5 0 0 1-1.5-1.5z" fill={paper} />
        <path class="paper" d="M3 10l9 6 9-6" fill="none" />
    {:else if name === 'flag'}
        <path class="red" d={flagCloth} fill={red} />
        <rect class="steel" x="4.5" y="2.5" width="2" height="19" rx="1" fill={steel} />
    {:else if name === 'flagOff'}
        <path class="hollow" d={flagCloth} />
        <rect class="steel" x="4.5" y="2.5" width="2" height="19" rx="1" fill={steel} />
    {:else if name === 'refresh'}
        {#each [0, 180] as turn (turn)}
            <g transform={`rotate(${turn} 12 12)`}>
                <path class="arc-edge" d="M4.8 10.5a7.3 7.3 0 0 1 12.2-3.8" />
                <path class="arc" d="M4.8 10.5a7.3 7.3 0 0 1 12.2-3.8" />
                <path class="green" d="M20.5 3v7h-7z" fill={green} />
            </g>
        {/each}
    {:else if name === 'inbox'}
        <path class="blue" d="M3 13.5 5.6 5.5a1.5 1.5 0 0 1 1.4-1h10a1.5 1.5 0 0 1 1.4 1L21 13.5v5a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5z" fill={blue} />
        <path class="paper" d="M3 13.5h5.5l1 2.5h5l1-2.5H21v5a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5z" fill={paper} />
    {:else if name === 'drafts'}
        <path class="paper" d="M6 2.5h8.5l5 5V20a1.5 1.5 0 0 1-1.5 1.5H6A1.5 1.5 0 0 1 4.5 20V4A1.5 1.5 0 0 1 6 2.5z" fill={paper} />
        <path class="paper" d="M14.5 2.5v5h5" fill="none" />
        <path class="rule" d="M7.5 11.5h9M7.5 14.5h9M7.5 17.5h6" />
    {:else if name === 'sent'}
        <path class="blue" d="M21.5 2.5 2.5 10.5l7 3 3 7.5z" fill={blue} />
        <path class="blue" d="M21.5 2.5 9.5 13.5" fill="none" />
    {:else if name === 'junk'}
        <circle class="red" cx="12" cy="12" r="9" fill={red} />
        <path class="shine" d="M5.2 10a7 7 0 0 1 13.6 0 12 12 0 0 0-13.6 0z" />
        <path class="ink" d="M6.8 6.8l10.4 10.4" />
    {:else if name === 'all'}
        <rect class="paper" x="6.5" y="3.5" width="15" height="10.5" rx="1.5" fill={paper} />
        <rect class="paper" x="2.5" y="8.5" width="15" height="11.5" rx="1.5" fill={paper} />
        <path class="paper" d="M3.2 9.5l6.8 5.5 6.8-5.5" fill="none" />
    {:else if name === 'plus'}
        <circle class="green" cx="12" cy="12" r="9" fill={green} />
        <path class="shine" d="M5.2 10a7 7 0 0 1 13.6 0 12 12 0 0 0-13.6 0z" />
        <path class="ink" d="M12 7.5v9M7.5 12h9" />
    {:else if name === 'attachment'}
        <path class="clip-edge" d="M16.5 8v8.5a4.5 4.5 0 0 1-9 0V6.5a3 3 0 0 1 6 0v9.5a1.5 1.5 0 0 1-3 0V8" />
        <path class="clip" d="M16.5 8v8.5a4.5 4.5 0 0 1-9 0V6.5a3 3 0 0 1 6 0v9.5a1.5 1.5 0 0 1-3 0V8" />
    {:else if name === 'image'}
        <rect class="paper" x="2.5" y="4.5" width="19" height="15" rx="1.5" fill={paper} />
        <rect x="4.5" y="6.5" width="15" height="11" fill={blue} />
        <path d="M4.5 17.5v-3l4-4 4.5 4.5 2.5-2.5 4 4v1z" fill={green} />
        <circle cx="15.5" cy="9.5" r="1.6" fill={gold} />
    {:else if name === 'download'}
        <path class="green" d="M9.5 3h5v7H18l-6 6.5L6 10h3.5z" fill={green} />
        <path class="clip-edge" d="M4 17v3h16v-3" />
        <path class="clip" d="M4 17v3h16v-3" />
    {:else if name === 'settings'}
        {#each [0, 45, 90, 135, 180, 225, 270, 315] as turn (turn)}
            <rect class="steel" x="10" y="1.5" width="4" height="5.5" rx="1" fill={steel} transform={`rotate(${turn} 12 12)`} />
        {/each}
        <circle class="steel" cx="12" cy="12" r="7" fill={steel} />
        <path class="shine" d="M6.3 10.5a5.9 5.9 0 0 1 11.4 0 11 11 0 0 0-11.4 0z" />
        <circle class="steel" cx="12" cy="12" r="2.6" fill={paper} />
    {:else if name === 'warning'}
        <path class="gold" d="M12 3 22 20.5H2z" fill={gold} />
        <path class="mark" d="M12 9.5v5M12 17.4v.2" />
    {/if}
</svg>

<style>
    .icon {
        flex: none;
        display: block;
        stroke-width: 1.1;
    }

    .glyph {
        fill: none;
        stroke: currentColor;
        stroke-width: 2.2;
    }

    .blue {
        stroke: var(--icon-blue-edge);
    }

    .green {
        stroke: var(--icon-green-edge);
    }

    .gold {
        stroke: var(--icon-gold-edge);
    }

    .red {
        stroke: var(--icon-red-edge);
    }

    .paper {
        stroke: var(--icon-paper-edge);
    }

    .steel {
        stroke: var(--icon-steel-edge);
    }

    .rule {
        fill: none;
        stroke: var(--icon-steel-dark);
        stroke-width: 1.3;
    }

    .groove {
        fill: none;
        stroke: var(--icon-steel-edge);
        stroke-width: 1.3;
    }

    .shine {
        fill: var(--icon-shine);
        stroke: none;
    }

    .ink {
        fill: none;
        stroke: var(--icon-ink);
        stroke-width: 2.6;
    }

    .mark {
        fill: none;
        stroke: var(--icon-gold-edge);
        stroke-width: 2.2;
    }

    .hollow {
        fill: none;
        stroke: currentColor;
        stroke-width: 1.5;
        opacity: 0.7;
    }

    .arc-edge {
        fill: none;
        stroke: var(--icon-green-edge);
        stroke-width: 4;
        stroke-linecap: butt;
    }

    .arc {
        fill: none;
        stroke: var(--icon-green-dark);
        stroke-width: 2;
        stroke-linecap: butt;
    }

    .clip-edge {
        fill: none;
        stroke: var(--icon-steel-edge);
        stroke-width: 3;
    }

    .clip {
        fill: none;
        stroke: var(--icon-steel-light);
        stroke-width: 1.2;
    }
</style>

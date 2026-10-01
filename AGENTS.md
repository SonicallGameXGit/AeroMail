# AGENTS.md

Single source of truth for any AI agent (or human) working on this project. Read it fully before changing anything. If a decision here turns out to be wrong, change this file in the same change that breaks the rule - never let the code and this file disagree.

Working name: **MailClient** (placeholder, final name not chosen).

## 1. What this is

A cross-platform email client in the category of Thunderbird, Spark, Mailspring.

Platform priority, in order:

1. **Desktop** - Windows, Linux (any distribution: Arch, Ubuntu, Fedora, ...), macOS; x86_64 and ARM64. Must be fully native-feeling: notifications, sounds, tray, autostart, deep links (`mailto:`), file associations, OS credential storage.
2. **Mobile** - Android and iOS.
3. **Web** - last and optional. Nice to have, never allowed to compromise 1 and 2.

## 2. Product philosophy

These two principles decide every design and feature argument. When unsure, re-read them.

### 2.1 Simple on the surface, deep underneath (the "Figma principle")

Figma is as approachable as Canva for someone who has never used a PC, and nearly as powerful as raw CSS. Rounding a square is one obvious gesture, yet nothing stops you from going much deeper. The opposite failures are:

- **Photoshop / Illustrator / GIMP**: powerful, but you need to be an expert to do a trivial thing.
- **Canva**: trivial things are easy, and then you hit a wall and resort to ugly workarounds.

Rules that follow:

- The common action is always one obvious step, with no configuration required first.
- Power is added by **progressive disclosure** (an "Advanced" section, a details panel, a context menu, a shortcut) - never by making the simple path longer.
- Never remove or hide a capability just to keep a screen clean. Find a deeper place for it instead.
- If a user needs a workaround to do something reasonable, that is a bug in the design.

### 2.2 Predictable core, optional smartness (the "C principle")

The base behaviour is plain, explicit and predictable, like C. Smart features are **extensions offered to the user**, never behaviour silently injected into the core. The user always keeps full control and can always predict what happens in their workspace.

Rules that follow:

- Every smart feature has a complete manual equivalent that works without it.
- Smart features suggest; the user confirms. Nothing "magic" happens without being visible and reversible.
- Smart features can be ignored or turned off entirely, and the app stays fully usable.
- A smart feature failing must never block the manual path.

Reference example (a real required feature, not an illustration): **account setup**.

- Smart path: user types an address at a well-known provider (gmail.com, outlook.com, ...), settings are detected automatically, connecting is one click - like Thunderbird.
- Manual path: full IMAP / SMTP / POP3 setup from scratch. Always reachable, for custom domains, **and** for well-known providers when the user prefers to do it by hand or when detection breaks. A detection failure leads into the manual form (pre-filled with whatever was found), plus a way to send feedback - never a dead end.

## 3. Feature scope

Baseline every mail client must have - all of it is in scope:

- Multiple accounts; IMAP, SMTP, POP3; OAuth2 where the provider requires it (Gmail, Outlook) and password / app-password auth.
- Folders / labels, unified inbox, threading, search.
- Read / unread, flag / star, archive, delete, move, spam, bulk actions, undo.
- Correct rendering of messages: plain text, HTML (sanitised), inline images, attachments, all charsets and encodings, sender / recipients / CC / BCC / headers view.
- Compose, reply, reply-all, forward, drafts, attachments, signatures, rich text and plain text.
- Offline access to synced mail, background sync, new-mail notifications with sound, tray icon with unread state.
- Settings, keyboard shortcuts, themes.

Smart, optional layer (examples; each must follow 2.2): provider autoconfiguration, and further ideas to be agreed with the owner before being built.

Do not invent large features on your own. Propose them.

## 4. Tech stack

| Layer | Choice | Why |
| --- | --- | --- |
| App shell | **Tauri 2** | One codebase for Windows / Linux / macOS / Android / iOS, native tray, notifications, small binaries. |
| Core | **Rust**, as a Cargo workspace of plain crates | All mail logic is testable with `cargo test`, with no UI and no Tauri. |
| UI | **Svelte 5 + SvelteKit** (static adapter, SPA mode), **TypeScript** strict | Owner's preferred stack. |
| Local storage | **SQLite** | Offline cache, search index (FTS5), settings. |
| Secrets | OS keychain (Secret Service / Keychain / Credential Manager / mobile equivalents) | Passwords and tokens never touch disk in plain text. |
| Package manager | **pnpm** | |

Library choices inside these layers (IMAP / SMTP / MIME parsing crates, HTML sanitiser, etc.) are made when the relevant module is first built: prefer maintained, pure-Rust, permissively licensed crates, check current state at adoption time, and record the choice in section 12.

Web target note: browsers cannot open raw IMAP / SMTP sockets, so a web build would need the Rust core running server-side behind the same API the UI already uses. Keep the UI ignorant of *how* it reaches the core (see 5.2) so this stays possible, but do not build it until asked.

## 5. Architecture

### 5.1 Layout

Exists today:

```
MailClient/
    AGENTS.md
    CLAUDE.md                   imports AGENTS.md
    Cargo.toml                  Cargo workspace (crates/*, src-tauri), shared lints
    .cargo/config.toml          where the generated TypeScript bindings go
    crates/
        mail-core/              domain types and rules: addresses, account settings, validation
        mail-autoconfig/        settings detection: built-in provider table, guess for unknown domains
    src-tauri/                  thin shell: commands forward to the crates; platform/ holds per-OS code
    src/                        SvelteKit UI (SPA, no SSR)
        lib/
            api/                the only place that talks to the core
                generated/      TypeScript types generated from Rust - never edit by hand
            design/             tokens.css, base.css and the primitives (Button, TextField, GlassWindow, ...)
            i18n/               `t()` and the dictionaries
            features/
                accounts/       account setup screen, its logic and tests
        routes/
```

Planned, created when first needed (do not add empty crates):

```
    crates/
        mail-protocols/         IMAP, SMTP, POP3 clients behind traits
        mail-store/             SQLite storage, migrations, search
        mail-render/            MIME parsing, HTML sanitising, charset handling
    tests/                      cross-crate integration tests, fixtures (.eml samples)
    docs/                       longer design notes when a topic outgrows this file
```

### 5.2 Rules

- **The core is headless.** Everything that is not presentation lives in `crates/`. No crate under `crates/` depends on Tauri or on the UI. If logic can be written in the core, it must be.
- **`src-tauri` is thin.** It converts commands and events and wires native features. No business logic.
- **The UI talks to the core through `src/lib/api` only.** Components never call Tauri `invoke` directly. This is what keeps mobile and a future web build cheap.
- **I/O behind traits.** Network, storage, clock and keychain are traits in the core, with real and fake implementations, so everything is testable without a network.
- **Types cross the boundary once.** Rust types that reach the UI derive `ts_rs::TS` with `#[ts(export)]`; `cargo test` writes them to `src/lib/api/generated`. Commit the generated files, never edit them, and re-export them from `src/lib/api/index.ts`.
- **Components get the API as a prop** (`api: Api`) instead of importing the Tauri implementation, so tests pass a fake (`features/accounts/fakeApi.ts`).
- **Manual path first.** For any smart feature, build the manual path first and the smart layer on top of it (2.2).
- **Reuse before writing.** Look for an existing helper, component or token before adding a new one. Small, composable modules over large ones.
- **Platform differences are isolated** in one module per concern (tray, notifications, sounds, keychain), with a common interface. No `cfg` / platform checks scattered through feature code.

### 5.3 Security and privacy

- Credentials and OAuth tokens only in the OS keychain. Never in SQLite, logs, config files, error messages or crash reports.
- Message HTML is untrusted input: sanitise it, render it isolated from the app (no script execution, no access to app APIs), block remote content by default with a visible per-message / per-sender opt-in.
- TLS is required by default; plaintext or invalid certificates need an explicit, clearly worded user decision.
- No telemetry and no network calls to anything but the user's mail providers and the documented autoconfig lookups, unless the owner asks for it and the user opts in.
- Never log message bodies or addresses at default log level.

## 6. Testing

Code is not done until it is tested. The goal is to verify behaviour in small parts **without launching the app and clicking buttons**.

- **Rust unit tests** next to the code for every parser, state machine and decision function.
- **Rust integration tests** for protocols against fakes and against a real local mail server in a container; sync engine against fake transports; storage against in-memory / temp SQLite.
- **Fixtures**: real-world-shaped `.eml` samples (odd charsets, nested multipart, broken HTML, huge attachments) under `tests/fixtures`. Every rendering or parsing bug fixed gets a fixture.
- **Autoconfig**: table-driven tests per provider, including the failure -> manual fallback path.
- **Frontend**: Vitest. `*.spec.ts` are plain unit tests (Node); `*.svelte.spec.ts` are component tests that run in a real headless Chromium with a fake `Api`. One-time setup: `pnpm exec playwright install chromium`.
- A bug fix comes with a test that fails before the fix.
- Never make a test pass by weakening or deleting it. If a test is wrong, say so and explain.

Before declaring work finished, run what applies and report the real result:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace      # also regenerates src/lib/api/generated
pnpm check                  # svelte-check, must be 0 errors
pnpm lint                   # ESLint, enforces the style in 7.1
pnpm test
```

Running the app: `pnpm tauri dev`. Linux needs the Tauri system libraries (WebKitGTK 4.1, GTK 3, libsoup 3, an appindicator library).

## 7. Code style

### 7.1 TypeScript / Svelte

- Strings use `'single quotes'`, even when the text contains `'` (escape it: `'don\'t'`).
- A string with a variable **inside** it uses a template literal: `` `Hello ${name}, welcome` ``. If the variable is only at the **start or end**, concatenate instead: `'Hello ' + name`, `count + ' unread'`.
- No semicolons. Only use `;` when several statements really have to share one line.
- Indent with 4 spaces.
- `PascalCase` for classes, enums and enum members; `camelCase` for variables and functions.
- Prefer procedural code: plain functions and data. Use a class or another paradigm when it genuinely fits the case better - then use it without hesitation.
- `const` everywhere it is possible. Exception: follow Svelte's own rules in `<script>` blocks, where `let` vs `const` changes reactivity / SSR behaviour.
- Arrays and objects that are too long for one line are split one element per line, indented one level, **with a trailing comma after the last element**. Never leave the last element without a comma in multi-line form.
- Every `if` has braces and a body on its own lines. Never brace-less, never inline:

```ts
if (condition) {
    ...
}
```

- **Zero type errors, and no lying to the compiler.** If a value is `unknown` or a union, narrow it with real checks (`typeof`, `in`, discriminants, guards) even when you "know" the runtime type. No `any`, no `as` casts to silence errors, no `@ts-ignore`.
- Check existence explicitly: `value === undefined`, `value === null`, `list.length === 0`. Not `!value`.

```ts
const describeUnread = (count: number, folder: string | undefined): string => {
    if (folder === undefined) {
        return count + ' unread'
    }

    return `${count} unread in ${folder} folder`
}

const defaultPorts = [
    993,
    465,
    587,
    995,
]
```

### 7.2 JSON

- Indent with 4 spaces.
- An array / object with 0-3 small elements is written inline; objects get a space after `{` and before `}`: `{ "host": "imap.example.com", "port": 993 }`, `[1, 2, 3]`.
- Anything bigger is split across lines, one level of indent per nesting.

### 7.3 Rust

- `rustfmt` defaults (4 spaces), `clippy` clean with `-D warnings`.
- No `unwrap` / `expect` / `panic!` outside tests; return typed errors. The workspace lints deny them, and every crate opts in with `[lints] workspace = true`.
- Errors are typed per crate and carry enough context to show the user something useful.
- Same spirit as TypeScript: plain functions and data first, traits where they buy testability or real polymorphism.

### 7.4 General

- Match the surrounding code: naming, comment density, idiom.
- Comments explain *why*, not *what*.
- User-facing text goes through `t()` from `src/lib/i18n`; no hard-coded UI strings in components. Typed keys built from a value (`` t(`protocol.${value}`) ``) are the one accepted exception to the concatenation rule in 7.1, because concatenation would lose the key type.

## 8. Visual design

**Not a default "modern flat / paper" look.** That generic AI-made style is explicitly rejected.

**Decided style** - three references, each with a fixed role:

| Reference | Role | Where it is used |
| --- | --- | --- |
| **Windows Aero** (Vista / 7) | The base style of the whole app | Window chrome, toolbars, panes, buttons, lists, menus, dialogs, inputs |
| **Frutiger Aero** (broad: sky, water, bokeh, bubbles, vivid blue-green) | Imagery only | App backdrop behind the glass, empty states, onboarding / account setup, splash |
| **Longhorn "Slate" / "Plex"** | The dark theme | Same components as Aero, re-toned dark and more serious |

What "Windows Aero" means concretely here:

- **Glass**: translucent, blurred, lightly tinted surfaces for chrome (title area, toolbars, sidebars). A thin bright inner border and a darker outer border give each glass panel an edge.
- **Content stays solid**: the message list and the reading pane sit on opaque (or near-opaque) surfaces. Glass is for chrome, never under long text.
- **Gloss**: buttons and selected items have a soft vertical gradient with a lighter upper half highlight; hover adds a glow, press darkens and flattens.
- **Depth**: soft drop shadows and glow separate layers; no hard flat blocks, no heavy black shadows.
- **Shape**: small radii (roughly 3-6px), not pills and not sharp squares.
- **Colour**: cool blue-led palette with green / aqua accents; saturated but airy. Slate theme: charcoal / graphite glass with the same accents, less saturation.
- **Icons**: detailed, glossy, slightly dimensional - not thin monochrome line icons.
- **Type**: a humanist sans in the Segoe UI / Frutiger spirit, bundled with the app so it looks the same on every OS. Not bundled yet: `--font-ui` is a system font stack for now; pick an openly licensed font and bundle it.
- **Motion**: short, soft fades and glows. Nothing bouncy.

Frutiger Aero imagery must never sit under the mail list or message text, and never be busy enough to compete with content.

Constraints:

- All colours, radii, shadows, blurs, gradients, spacing and motion come from tokens in `src/lib/design`. No hard-coded visual values in feature components.
- The style serves section 2: an affordance must look like what it does. Decoration never costs clarity or speed.
- Glass / blur effects are drawn **inside the app** over the app's own backdrop, so they look the same on every OS. Real behind-window blur is not available consistently (notably on Linux), so it may only ever be an optional enhancement.
- Readable text contrast (WCAG AA) on every surface, including glass. Full keyboard navigation. Respect reduced-motion and reduced-transparency settings.
- Light and dark variants are designed together.
- Must stay fast on low-end hardware: effects degrade gracefully.
- Desktop and mobile share tokens and components; layouts adapt, they are not separate designs.

## 9. Native integration checklist (desktop)

Each must work on Windows, Linux and macOS, behind one interface (5.2):

- Notifications for new mail, with actions where the OS supports them.
- Sounds (user-configurable, can be disabled).
- Tray icon with unread indicator and menu; close-to-tray is a setting.
- Autostart, single instance, `mailto:` handler, opening `.eml` files.
- OS keychain.
- Linux: must not assume a specific desktop environment, display server (X11 and Wayland) or distribution; degrade gracefully when a service (tray, Secret Service) is missing, and tell the user.

## 10. How to work here

- Understand before editing: read the relevant code and this file. Do not guess at conventions.
- Keep changes small and focused. No drive-by refactors or unrequested features.
- Ask the owner before: adding a dependency of significant size, changing architecture rules, changing anything in sections 2 and 8, or anything hard to undo.
- Do not commit or push unless asked.
- Report honestly: what was done, what was verified and how, what was not. Failing tests are reported as failing.
- Update this file when a decision is made or changed (section 12).

## 11. Roadmap (high level)

1. Agent docs (this file).
2. Decide visual style (section 8) - done.
3. Scaffold workspace: crates, Tauri shell, SvelteKit UI, test harness - done. CI on all three desktop OSes - not done.
4. Core, headless and tested: accounts -> manual IMAP / SMTP / POP3 -> storage -> sync -> MIME / rendering. Done so far: account settings types and offline validation. **Next: real connections** (IMAP / SMTP / POP3 login with a password kept in the OS keychain), then OAuth2 for Gmail and Outlook.
5. Autoconfig on top of manual setup. Done: built-in provider table with a guess fallback. Later: online lookups (provider autoconfig files, DNS SRV) behind a trait.
6. Design system, then UI: account setup, mail list, reader, composer, settings. Done so far: tokens for both themes, basic primitives, the account setup screen up to checking settings (it does not connect yet).
7. Native integration: notifications, sounds, tray, autostart, `mailto:`.
8. Mobile.
9. Web (optional).

## 12. Decision log

Newest last. One line per decision: date, decision, reason.

- 2026-10-01 - Stack proposed: Tauri 2 + Rust core crates + Svelte 5 / SvelteKit + SQLite + pnpm. Reason: desktop-first with mobile from one codebase, headless testable core. Confirmed by owner.
- 2026-10-01 - Visual style confirmed by owner: Windows Aero as base, Frutiger Aero imagery for backdrops / empty states / onboarding only, Longhorn Slate as the dark theme. Glass is drawn in-app so it is identical on every OS.
- 2026-10-01 - TypeScript bindings generated with `ts-rs` on `cargo test`. Reason: one source of truth for types, no extra build step.
- 2026-10-01 - Code style enforced by ESLint (`@stylistic`) rather than by review. Reason: every agent gets the same result.
- 2026-10-01 - Unknown domains produce a guessed, editable pre-fill (`imap.` / `smtp.` / `pop.` + domain) instead of an error. Reason: section 2.2, detection failure must lead into the manual form.
- 2026-10-01 - Linux startup sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` only on Wayland with the proprietary NVIDIA driver, unless the user set it (`src-tauri/src/platform/linux.rs`). Reason: WebKitGTK otherwise dies with "Error 71 (Protocol error) dispatching to Wayland display".
- 2026-10-01 - Dev server port is 5175 (`vite.config.ts` and `devUrl` in `src-tauri/tauri.conf.json` must match). Reason: 5173 is used by another project on the owner's machine.

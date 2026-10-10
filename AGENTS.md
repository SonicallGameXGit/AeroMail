# AGENTS.md

Single source of truth for any AI agent (or human) working on this project. Read it fully before changing anything. If a decision here turns out to be wrong, change this file in the same change that breaks the rule - never let the code and this file disagree.

Working name: **MailClient** (placeholder, final name not chosen).

## 1. What this is

A cross-platform email client in the category of Thunderbird, Spark, Mailspring.

Platform priority, in order:

1. **Desktop** - Windows, Linux (any distribution: Arch, Ubuntu, Fedora, ...), macOS; x86_64 and ARM64. Must be fully native-feeling: notifications, sounds, tray, autostart, deep links (`mailto:`), file associations, OS credential storage.
2. **Web** - the same UI in a browser, with the Rust core running server-side. Never allowed to compromise 1.
3. **Mobile** - Android and iOS, as separate native apps with the same design. Not built from this codebase's UI; nothing here is bent to make that possible.

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

Smart, optional layer (examples; each must follow 2.2): provider autoconfiguration, sender pictures (a brand's own logo next to its mail; a coloured initial is the manual equivalent and the fallback), and further ideas to be agreed with the owner before being built.

Do not invent large features on your own. Propose them.

## 4. Tech stack

| Layer | Choice | Why |
| --- | --- | --- |
| App shell | **Electron** | Ships its own Chromium, so the app renders the same on every desktop, whatever web engine and GPU driver the system has. Native tray, notifications, dialogs. The cost (a larger download, more memory) is accepted. |
| Core | **Rust**, as a Cargo workspace of plain crates | All mail logic is testable with `cargo test`, with no UI and no shell. The shell runs it as a child process. |
| UI | **Svelte 5 + SvelteKit** (static adapter, SPA mode), **TypeScript** strict | Owner's preferred stack. |
| Local storage | **SQLite** | Offline cache, search index (FTS5), settings. |
| Secrets | OS keychain (Secret Service / Keychain / Credential Manager / mobile equivalents) | Passwords and tokens never touch disk in plain text. |
| Package manager | **pnpm** | |

Library choices inside these layers (IMAP / SMTP / MIME parsing crates, HTML sanitiser, etc.) are made when the relevant module is first built: prefer maintained, pure-Rust, permissively licensed crates, check current state at adoption time, and record the choice in section 12.

Web target note: browsers cannot open raw IMAP / SMTP sockets, so a web build needs the Rust core running server-side behind the same API the UI already uses. `mail-host::dispatch` is that API with no transport attached: the desktop feeds it from stdin, a web server would feed it from requests (and needs its own answer for secrets and for several users). Keep the UI ignorant of *how* it reaches the core (see 5.2) so this stays cheap, but do not build it until asked.

## 5. Architecture

### 5.1 Layout

```
MailClient/
    AGENTS.md
    CLAUDE.md                   imports AGENTS.md
    Cargo.toml                  Cargo workspace (crates/*), shared lints
    .cargo/config.toml          where the generated TypeScript bindings go
    crates/
        mail-core/              domain types and rules (incl. which sender logos are acceptable, which providers have a sign-in page, and `AppSettings`); the traits for servers, keychain, logos and sign-in; MailError
        mail-autoconfig/        settings detection: built-in provider table, guess for unknown domains; what a half-typed address says (provider mark, typo)
        mail-protocols/         real IMAP and SMTP behind the core traits (NetConnector); DNS + HTTPS lookup of sender logos (NetLogos); sign-in on the provider's page (NetSignIn)
        mail-store/             SQLite: accounts, folders, cached messages, settings, looked-up sender logos
        mail-render/            header parsing, HTML sanitising, attachments, composing raw messages
        mail-engine/            the operations the UI calls; owns the store, uses the traits
        mail-host/              the engine as a process (`mailclient-core`): commands as JSON, keychain
    electron/                   thin shell: window, starts the core, save dialog, external links, notifications
        icons/
    electron-builder.json       packaging: installers with the core binary next to the app
    .env.example                what a build can be given (copied to the git-ignored `.env`)
    scripts/                    what `pnpm dev`, `pnpm build` and `pnpm test:app` run
    tests/
        fixtures/               `.eml` samples, one per rendering or parsing bug fixed
    src/                        SvelteKit UI (SPA, no SSR)
        lib/
            api/                the only place that talks to the core (`bridge.ts`: what the shell exposes)
                generated/      TypeScript types generated from Rust - never edit by hand
            design/             tokens.css, base.css, the themes (`theme.ts`) and the primitives (Button, Icon, Avatar, BrandMark, Menu, Splitter, TextField, CheckField, GlassWindow, Modal, MessageBox, MessageIcon, ThemePreview, ...)
            i18n/               `t()` and the dictionaries
            features/
                accounts/       account setup screen, its logic, the fake Api for tests
                notifications/  what a new-mail notification says and shows (templates, picture), and its settings
                settings/       the settings window: categories in a list on the left (accounts, appearance, privacy, notifications), one shown at a time
                workspace/      account menu, folders, message list, reader with its actions, composer
        routes/
```

Dependency direction: `mail-core` <- everything; `mail-engine` uses `mail-store` and `mail-render` directly and reaches servers and the keychain only through the traits in `mail-core::transport`; only `mail-host` knows `mail-protocols` and the keychain library. Schema changes are added as a new step in `Store::prepare` (`user_version`), never by editing an older step: existing databases must keep opening.

Planned, created when first needed (do not add empty crates or folders):

```
    tests/                      cross-crate integration tests (only `fixtures/` exists so far)
    docs/                       longer design notes when a topic outgrows this file
```

### 5.2 Rules

- **The core is headless.** Everything that is not presentation lives in `crates/`. No crate under `crates/` depends on Electron or on the UI. If logic can be written in the core, it must be.
- **The shell is thin.** `electron/` opens the window, runs the core and wires native features. No business logic, and nothing in TypeScript that could live in Rust.
- **One way from the UI to the core.** The page calls `window.mailclient` (exposed by `electron/preload.cts`), the main process passes the command to `mailclient-core` as one JSON line on its stdin and returns the answer line: `{ id, ok, value }` or `{ id, ok, error }`. The core also speaks unasked, on the same stdout: `{ event }` lines (no `id`, type `MailEvent`) that the shell passes on to the page (`onEvent`). No local port is opened for this (the only listener the app ever has is the short-lived sign-in one in 5.3, which serves the browser, not the UI). A new operation is a new arm in `mail-host::dispatch` plus a method in `src/lib/api`.
- **The UI talks to the core through `src/lib/api` only.** Components never touch `window.mailclient` directly. This is what keeps a web build cheap.
- **I/O behind traits.** Mail servers, the keychain, the sender-logo lookup and the provider sign-in are traits in `mail-core::transport`, with real and fake implementations, so the engine is testable without a network. Storage is not behind a trait: it is always SQLite, and tests use it in memory.
- **One error type crosses boundaries**: `MailError { kind, server, detail }`. The UI words the message from `kind` and `server`; `detail` is the technical line. Add an `ErrorKind` rather than encoding meaning in `detail`.
- **Database ids are never reused** (`AUTOINCREMENT`): the UI and the keychain hold ids.
- **Types cross the boundary once.** Rust types that reach the UI derive `ts_rs::TS` with `#[ts(export)]`; `cargo test` writes them to `src/lib/api/generated`. Commit the generated files, never edit them, and re-export them from `src/lib/api/index.ts`.
- **Components get the API as a prop** (`api: Api`) instead of importing the desktop implementation, so tests pass a fake (`features/accounts/fakeApi.ts`).
- **Manual path first.** For any smart feature, build the manual path first and the smart layer on top of it (2.2).
- **Reuse before writing.** Look for an existing helper, component or token before adding a new one. Small, composable modules over large ones.
- **Platform differences are isolated** in one module per concern (tray, notifications, sounds, keychain), with a common interface. No `cfg` / platform checks scattered through feature code.

### 5.3 Security and privacy

- Credentials and OAuth tokens only in the OS keychain (service `app.mailclient.desktop`, key `account-<id>`: the password, or the refresh token of an account that signs in on its provider's page). Never in SQLite, logs, config files, error messages or crash reports. Access tokens live only in the core's memory.
- **Sign-in on the provider's page** (OAuth2, Google so far) follows the rules for installed apps, and all of these must stay: the page opens in the user's own browser, never in a window of the app, so the app never sees the password; the provider answers to a listener on `127.0.0.1` only, on a port picked by the system, which exists only while a sign-in waits (five minutes at most, one at a time) and accepts nothing but the answer carrying this sign-in's random `state`; the code is traded in with PKCE (S256); tokens are requested over HTTPS with the app's usual certificate checks. When the user picks the account on the provider's page instead of typing it, the app also asks for `openid email`, and takes the address from the ID token in the token answer only when it was made for this app (`aud`) and the provider marks the address verified; its signature is not checked, because it came straight from the token address over a verified TLS connection (OpenID Connect Core 3.1.3.7). Nothing else from the token is used or kept. The provider is chosen by the mail server's host name, from the table in `mail-core::oauth`, never from anything a server or a message says. The client id and "client secret" identify the app, not the user; they are given to the build (see 6) and are not in the repository.
- Message HTML is untrusted input, with two layers that must both stay: `mail-render` sanitises it (and strips remote images unless allowed), and the UI shows it in an `<iframe sandbox="allow-same-origin">` (never `allow-scripts`) whose own content policy allows only inline styles, images and embedded fonts, and, once remote content is allowed, the message's remote images, fonts and imported style sheets (never scripts). The app page's policy allows those too, because the frame inherits it and can only narrow it. Remote content is blocked by default with a visible per-message opt-in; the user can allow remote images for every message in the settings (`AppSettings.remote_images`, off by default, worded with what it gives away), and the core, not the UI, decides what a message is rendered with. Links are caught from outside the frame and opened in the system browser, `http(s)` and `mailto` only.
- The window runs with `contextIsolation`, `sandbox` and no Node integration; it cannot navigate away or open windows. The page's own policy (`csp` in `vite.config.ts`) allows no remote scripts. The shell opens only `http(s)` and `mailto` links, checked again in the main process.
- TLS certificates are verified against the bundled Mozilla roots (`webpki-roots`); there is no "accept anyway" path yet.
- TLS is required by default; plaintext or invalid certificates need an explicit, clearly worded user decision.
- No telemetry and no network calls to anything but the user's mail providers (their sign-in and token addresses included), the documented autoconfig lookups and the sender-picture lookup below, unless the owner asks for it and the user opts in.
- **Sender pictures** (BIMI) are on by default and can be turned off in the settings; turned off, nothing is looked up. For a sender's domain (and up to two parent domains) the core asks DNS for `default._bimi.<domain>` and `_dmarc.<domain>`, then downloads the logo over HTTPS. Nothing about the user or the message is sent, but the logo's host sees the user's IP address. Everything in the record is the sender's and untrusted, so all of these must stay: a logo is used only if the domain's DMARC policy refuses forged mail (`quarantine` / `reject`); only plain `https` to a public host name, default port, no redirects, no proxy; the host's addresses are resolved first, private / local ones are dropped and the download is pinned to the rest; at most 512 KB, SVG only; shown only through `<img>` as a `data:` address (never inlined into the page), where it cannot run scripts or load anything. Answers, including "no logo", are cached per domain for 7 days. The certificate that proves a logo belongs to the brand (VMC) is **not** checked yet, so a look-alike domain with its own DMARC policy can publish someone else's logo: a picture is decoration, never a sign that a message is genuine.
- Never log message bodies or addresses at default log level.

## 6. Testing

Code is not done until it is tested. The goal is to verify behaviour in small parts **without launching the app and clicking buttons**.

- **Rust unit tests** next to the code for every parser, state machine and decision function.
- **Engine tests** (`mail-engine/src/tests.rs`) run every operation against the in-memory fakes in `testing.rs` and a real in-memory SQLite. New engine behaviour gets a test here first.
- **Protocol tests** (`mail-protocols`) run the real IMAP and SMTP clients against scripted servers on local sockets (`fake_server.rs`). No container needed.
- **Opt-in tests** (`#[ignore]`): `cargo test -p mail-protocols -- --ignored` checks TLS and STARTTLS against real public servers with a bogus login and a bogus access token, and the sender-logo lookup against real domains; `cargo test -p mail-host -- --ignored` round-trips through the real OS keychain. Run them after touching TLS, connection or keychain code.
- **Fixtures**: real-world-shaped `.eml` samples (odd charsets, nested multipart, broken HTML, huge attachments) under `tests/fixtures`. Every rendering or parsing bug fixed gets a fixture.
- **Autoconfig**: table-driven tests per provider, including the failure -> manual fallback path.
- **Frontend**: Vitest. `*.spec.ts` are plain unit tests (Node); `*.svelte.spec.ts` are component tests that run in a real headless Chromium with a fake `Api`. One-time setup: `pnpm exec playwright install chromium`.
- **Shell**: `pnpm test:app` starts the real Electron app with the built UI and an empty throwaway data folder, and checks that the page and the setup form reach the core and that its content policy lets the message frame load. It uses its own browser profile, so it also runs while the app is open. It needs a display (`xvfb-run -a pnpm test:app` without one). Run it after touching `electron/`, `mail-host` or the policy.
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

Running the app: `pnpm dev` (builds the core and the shell, then opens the app with hot reload for the UI). Installer for the current system: `pnpm build`, written to `release/`. These are the only two ways to run and build; do not add variants.

Signing in with Google needs the owner's OAuth client (Google Cloud console, client type "Desktop app", scope `https://mail.google.com/`): put `MAILCLIENT_GOOGLE_CLIENT_ID` and `MAILCLIENT_GOOGLE_CLIENT_SECRET` into `.env` (copy `.env.example`; `.env` is never committed). `scripts/run.js` loads it for `pnpm dev`, `pnpm build` and `pnpm test:app`, a variable set in the terminal wins, and the values are compiled into the core. A plain `cargo` command does not read `.env`. A build made without them still works: it suggests an app password for Gmail, as before.

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
- **Icons**: detailed, glossy, slightly dimensional - not thin monochrome line icons. They are drawn in `design/Icon.svelte` from a few glossy materials (tokens `--icon-*`); only small structural glyphs (chevron, close, check) are flat.
- **Pictures before words**: an action or a place is recognised by its icon first. A button keeps its text only when it is the main action of its group (New message, Reply); every icon-only control has a tooltip and an accessible name.
- **Buttons say what they are**: accent (the one main action), normal, tool (flat until hovered, for toolbars), danger (red, for what destroys something).
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

## 11. Roadmap and current state

Works today (IMAP + SMTP, password sign-in):

- Account setup: detection, manual form, real sign-in to both servers, password in the keychain; while the address is typed, Gmail's mark shows inside the field, and the Gmail name with another ending (`gmail.ru`) hides it and asks whether the address is typed right, with a one-click fix; an arrow at the end of the field (the provider's mark takes its place) lists the well-known providers, and picking one fixes its domain after the field (`john` + a dimmed `@gmail.com`), "None" first and chosen by default; "Set up manually" on the first step only for a provider with a mark (an unknown domain opens the form anyway); several accounts; a message box says the account was added (and, when it was opened, the settings close before it shows); removing an account. Gmail (and custom domains on Google's servers) signs in on Google's own page with one click, in a build given a client id (6); an app password stays one click away. In such a build the first step also offers "or using" with a Gmail button: Google's page then lists the accounts signed in to the browser, and the one picked is added as if its address had been typed. Adding an account that is already in the app says so (`ErrorKind::Duplicate`).
- Workspace layout: account menu at the top right (switch, add, open the settings); folders of the open account on the left with New message above them; message list; reader with the actions of the open message above it, grouped as answer / put away / mark. Panes are resizable, the folder list down to an icon rail.
- Folders with roles and unread counts; message list (newest 50, "load older"); cached for offline reading; refresh on demand and every 2 minutes for the open folder.
- Sender pictures: a brand's own logo (BIMI) under the same glass as the initials, cached, with a switch in the settings.
- Reading: plain text and sanitised HTML, inline images, remote images on request (or always, by a setting), attachments saved to disk.
- Settings window (opened from the account menu): a list of categories on the left, the open one on the right - Accounts (add, remove), Appearance (theme: Automatic, Light - Aero, Dark - Slate, each shown as a small preview), Privacy (sender pictures, always show remote images), Notifications. Changes apply at once.
- New mail arrives by itself: the core watches every account's inbox (IMAP IDLE on a connection of its own, a check every minute for servers without it), the open list updates without a refresh, and the system shows a notification (by default the sender's picture, their name, the subject and the start of the message; a click opens the message). In the settings: on / off and sound at the top, with a live sample; "Customize the notification" opens title and text (ready-made choices or the user's own text with fields), length of the message part, picture (sender, app icon, the user's own, none), how long it stays, and whether it shows while the app is in front.
- Read / unread, flag, delete (to Trash, final inside Trash), archive, move to folder.
- Writing: new message, reply, reply all, forward (plain text), Cc / Bcc, copy saved to Sent.

Not built yet, roughly in this order:

1. **OAuth2** for Outlook (Outlook.com accepts nothing else): an entry in `mail-core::oauth` and a client id registered by the owner. Signing in again for an account whose permission ran out or was taken back (today: remove the account and add it again). Google's verification of the app (until then Google shows an "unverified app" warning, allows only listed test users and ends each permission after 7 days).
2. **POP3** (the form offers it; connecting answers "not available yet").
3. Search, threading, unified inbox, multi-select and bulk actions, undo, drafts, signatures.
4. Composer: attachments, rich text. Marking the original as answered after a reply.
5. Native integration: notification sounds of our own, tray with an unread count, autostart, `mailto:`, `.eml`. Watching folders other than the inbox (they are still checked every 2 minutes while open).
6. More in the settings window (pane widths, refresh interval, sounds, close-to-tray), keyboard shortcuts, bundled font.
7. Online autoconfig lookups (provider autoconfig files, DNS SRV) behind a trait.
8. Self-signed / private-CA certificates with an explicit user decision.
   Sender pictures: check the logo's certificate (VMC), and the message's own `Authentication-Results`, before showing a logo.
9. CI on Windows, Linux and macOS; installers checked on each (only the Linux build was run so far), code signing.
10. Web: an HTTP front for `mail-host::dispatch` and a matching `Api` in the UI.
11. Mobile: separate native apps.

## 12. Decision log

Newest last. One line per decision: date, decision, reason.

- 2026-10-01 - Stack proposed: Tauri 2 + Rust core crates + Svelte 5 / SvelteKit + SQLite + pnpm. Reason: desktop-first with mobile from one codebase, headless testable core. Confirmed by owner.
- 2026-10-01 - Visual style confirmed by owner: Windows Aero as base, Frutiger Aero imagery for backdrops / empty states / onboarding only, Longhorn Slate as the dark theme. Glass is drawn in-app so it is identical on every OS.
- 2026-10-01 - TypeScript bindings generated with `ts-rs` on `cargo test`. Reason: one source of truth for types, no extra build step.
- 2026-10-01 - Code style enforced by ESLint (`@stylistic`) rather than by review. Reason: every agent gets the same result.
- 2026-10-01 - Unknown domains produce a guessed, editable pre-fill (`imap.` / `smtp.` / `pop.` + domain) instead of an error. Reason: section 2.2, detection failure must lead into the manual form.
- 2026-10-01 - Linux startup sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` only on Wayland with the proprietary NVIDIA driver, unless the user set it (`src-tauri/src/platform/linux.rs`). Reason: WebKitGTK otherwise dies with "Error 71 (Protocol error) dispatching to Wayland display".
- 2026-10-01 - Dev server port is 5175 (`vite.config.ts` and `devUrl` in `src-tauri/tauri.conf.json` must match). Reason: 5173 is used by another project on the owner's machine.
- 2026-10-01 - Mail libraries: `async-imap`, `lettre` (SMTP), `mail-parser`, `mail-builder`, `ammonia`, `rusqlite` (bundled SQLite), `keyring` 3, TLS through `rustls` + `webpki-roots`. Reason: maintained, pure Rust TLS (no OpenSSL to ship on any platform).
- 2026-10-01 - Sync model: the folder's full UID list is fetched, a window of the newest messages is cached and reconciled (new, gone, flags); bodies are downloaded on first open and kept. Reason: simple and correct; revisit (CONDSTORE, IDLE) when large mailboxes or push matter.
- 2026-10-01 - One password is used for both the incoming and the outgoing server of an account. Reason: true for practically every provider; split it when a real case appears.
- 2026-10-01 - Gmail is listed with password sign-in (app password) until OAuth2 exists. Reason: it is the only way that works today.
- 2026-10-02 - Platform priority changed to Desktop > Web > Mobile; mobile becomes separate native apps with the same design. Reason: owner's decision - one UI everywhere was costing the desktop.
- 2026-10-02 - Shell changed from Tauri 2 to Electron. Reason: WebKitGTK made the Linux app slow and unstable (resizing redrew through the CPU; NVIDIA on Wayland crashed without a workaround that disabled the GPU path); a bundled Chromium behaves the same on every system. Tauri's own Chromium (CEF) runtime was still in development.
- 2026-10-02 - The Rust core runs as a child process (`mailclient-core`) speaking JSON lines over stdin / stdout, not as a Node native module. Reason: no local port, a core crash cannot take the window down, and the same `dispatch` can sit behind a web server later.
- 2026-10-02 - The data folder (`app.mailclient.desktop` in the OS data directory) and the keychain names are unchanged from the Tauri build. Reason: accounts already added keep working.
- 2026-10-02 - The shell is compiled with plain `tsc` (the preload is a `.cts` file) and packaged with `electron-builder`. Reason: no bundler to configure for three small files.
- 2026-10-02 - The WebKitGTK `WEBKIT_DISABLE_DMABUF_RENDERER` workaround is gone with Tauri, and the dev server port (5175) now lives only in `vite.config.ts`; the shell is told the address by `scripts/dev.js`.
- 2026-10-02 - One command to run (`pnpm dev`) and one to build (`pnpm build`); the steps live in `scripts/`, not in a chain of package scripts. Reason: owner wants one obvious way, no variants.
- 2026-10-02 - Workspace laid out by what things belong to: accounts in a menu at the top right, New message and folders in the sidebar, message actions above the open message (not in one global toolbar). One account's folders are shown at a time. Reason: owner found the single row of identical text buttons and the text-only sidebar hard to read.
- 2026-10-02 - Icons are hand-drawn SVG inside `Icon.svelte`, no icon library. Reason: no maintained set matches the glossy Aero look, and line icons are rejected by section 8.
- 2026-10-02 - Well-known folders are shown first in one fixed order (Inbox, Flagged, Drafts, Sent, Archive, Junk, Trash, All mail), the user's own folders below in the server's order. Reason: a folder is found by position and icon, the same on every provider.
- 2026-10-02 - Pane widths are kept in the page's `localStorage` (`workspace.layout`), not in the core. Reason: they belong to this window on this machine; move them when the settings screen exists.
- 2026-10-02 - Menus use the browser's popover layer and are positioned from their button. Reason: nothing can clip them, and Escape / click-outside come for free.
- 2026-10-02 - Component tests run in a 1280x800 viewport. Reason: the workspace is a desktop window; at a phone size its panes do not fit.
- 2026-10-02 - Sender pictures come from BIMI only (no website icons, no Gravatar), on by default with a switch. Reason: owner's choice - official logos only, no third party learns who writes to the user; section 5.3 lists the lookup and its limits.
- 2026-10-02 - Libraries for it: `hickory-resolver` (DNS TXT, system resolver settings) and `reqwest` without default features, given the app's own `rustls` setup (ring, bundled Mozilla roots). Reason: maintained, pure Rust, and TLS stays configured in one place (`mail-protocols::tls`).
- 2026-10-02 - The organisation's domain is approximated by walking up to two parent domains instead of using the public suffix list. Reason: no extra data to ship; a lookup on a bare suffix like `co.uk` simply finds nothing.
- 2026-10-02 - Avatars (initials and logos alike) sit under one glass overlay drawn by `Avatar.svelte`. Reason: foreign logos look part of the theme without being altered.
- 2026-10-02 - The settings table (`settings`, name / value) exists in SQLite from this change; `sender_pictures` is its first entry.
- 2026-10-02 - The sanitiser keeps `class` and `id`, and the message frame adds only defaults the message's own style sheet overrides (no image size limits). Reason: newsletters are laid out by class rules in their `<style>` block and lost their paddings and font sizes; the frame runs no scripts and holds only the message, so these attributes cannot reach the app.
- 2026-10-02 - Settings are one value, `AppSettings`, read and written whole (`settings` / `set_settings`), replacing the two sender-picture commands; in SQLite each field stays its own `settings` row holding "on" / "off", a missing row meaning the default. Reason: a new switch is a field, not two more commands, and stored choices keep working.
- 2026-10-02 - "Always show remote images" exists as a setting, off by default; the per-message opt-in stays for everyone who leaves it off. Reason: owner's request; section 2.2 - the user decides, knowing what it gives away.
- 2026-10-02 - Removing an account moved from a bare trash icon in the account menu to the settings window, as a labelled button next to each account. Reason: the owner did not find it.
- 2026-10-02 - Gmail signs in on Google's page (OAuth2): system browser, answer to a listener on 127.0.0.1 with a random port, PKCE; all of it in the core behind `SignInFlow`, the shell only opens the address. Reason: Google refuses sign-in inside an app's own web view; this is the flow it documents for installed apps. Replaces "Gmail is listed with password sign-in" above.
- 2026-10-02 - The app password stays as the manual path: "Use a password instead" on the setup screen, and what detection suggests by itself in a build without a client id. Reason: section 2.2.
- 2026-10-02 - Google's client id and secret come from the build's environment (`MAILCLIENT_GOOGLE_CLIENT_ID`, `MAILCLIENT_GOOGLE_CLIENT_SECRET`), kept in a git-ignored `.env` that the scripts load, not from the repository. Reason: they belong to the owner's Google project; a desktop app cannot keep them secret, but they need not be published either.
- 2026-10-02 - The provider to sign in with is found by the mail server's host name (`imap.gmail.com`, ...), not by the address's domain. Reason: custom domains hosted by Google work through the manual form.
- 2026-10-02 - For an account that signs in on its provider's page, the keychain entry holds the refresh token; access tokens are kept in memory and renewed a minute before they run out. Both servers of an account sign in the same way (`IssueKind::MixedAuth`). Reason: one secret per account, as with passwords.
- 2026-10-02 - Adding such an account is two commands, `begin_sign_in` (gives the address to open) and `finish_sign_in` (answers when the user is done), plus `cancel_sign_in`. Reason: the core cannot open a browser, and one request has one answer.
- 2026-10-02 - The page the browser shows after signing in is plain English text sent by the core, outside `t()`. Reason: it is not part of the UI; move it when a second language exists.
- 2026-10-02 - `ErrorKind::SignIn` added for a sign-in that was not completed or a permission that is no longer valid. `reqwest` gains its `form` feature; `ring` and `base64` (already in the tree) became direct dependencies of `mail-protocols` for PKCE.
- 2026-10-02 - `src/lib/api` turns the parameters of every command into plain data (a JSON round trip) before handing them to the shell. Reason: components pass Svelte `$state`, which is a proxy that Electron cannot copy ("An object could not be cloned"); no account could be added since the move to Electron. `pnpm test:app` now submits the setup form to catch this.
- 2026-10-02 - Speed was measured against Gmail before changing anything: every IMAP command costs 120-140 ms there whatever it asks, so the number of commands is what a sync or an opened message costs. Decided from that: a connection that answered within the last 30 seconds is not asked `NOOP` before the next operation (`mail-protocols::alive`; a failed operation makes the next one check and reconnect); the message list fetches only the header fields it shows (Subject, From, To, Cc, Date), not whole header blocks; mail sockets set `TCP_NODELAY`; the refresh button syncs the open folder before it counts the others.
- 2026-10-02 - Dependencies are optimised in the dev profile (`[profile.dev.package."*"]`), the project's own crates are not. Reason: `pnpm dev` was about three times slower than an installed build at syncing and opening mail; libraries compile once, so rebuilds stay fast.
- 2026-10-07 - The message frame has `color-scheme: light`, so a message's `prefers-color-scheme: dark` rules stay off. Reason: the frame is a white page in every theme (as in Gmail), and messages that declare `color-scheme: light dark` turned their text white on it in the dark theme. Showing messages dark is a separate decision: it would change the page and the scheme together.
- 2026-10-07 - The message's `<body>` attributes are kept: the tag is taken out before sanitising, its attributes are sanitised on a stand-in `<div>` (`bgcolor` and `text` become style), and the result is written back as a `<body>` tag at the start of the HTML, which the browser merges into the frame's own body. `<style media>` is kept. A blocked remote image points at `data:,` instead of losing its address. Reason: newsletters set their background, margins, font and colour on `<body>`, a style sheet for one screen size applied to all, and an `<img>` without an address is drawn as an empty bordered box. Measured on the 30 cached messages: all now render the same as their unsanitised HTML (offline, light), against 11 before.
- 2026-10-07 - Allowed remote content includes the message's web fonts and imported style sheets (`font-src` / `style-src` with `http: https:` in the frame only after the opt-in; `@import` counts as remote content). Reason: newsletters bring their typeface from their own servers or Google Fonts; without it a Linux system falls back to Liberation Sans, which has no medium or semibold weights, so headings looked thin. Same privacy cost as remote images, behind the same opt-in, and the wording now says fonts. `<link>` style sheets stay removed by the sanitiser.
- 2026-10-07 - New mail is found by the core, not the page: `Engine::watch` runs one task per account that syncs the inbox, then waits on a connection of its own (`IncomingSession::wait_for_change`: IMAP IDLE for 5 minutes at a time, a 60 second pause on servers without IDLE), and retries with a growing pause (5 s to 5 min) when offline. Reason: it works while no page is looking, and one connection waiting never blocks a click.
- 2026-10-07 - Announcements are `MailEvent` (`FolderChanged`, `NewMail`), sent by `sync_folder` itself, whoever asked for the sync, so a message is announced exactly once. "New" means unread, in an inbox, with a UID above everything already cached: the first sync of a folder, loading older mail and mail read elsewhere are not news. Known gap: the very first message into a never-synced, empty inbox is not announced.
- 2026-10-07 - Notification text is worded by the UI (`t()`, `newMailNotification`) and shown by the shell (`shell:notify`, Electron's `Notification`, which uses the native service on Windows, macOS and Linux) with the app icon; it is skipped while the window is in front. Sender pictures are not used as the icon: Electron cannot draw SVG into one. `AppSettings.notifications` (on by default) is checked by the core, so with it off the page hears only `FolderChanged`.
- 2026-10-09 - New mail reaches the notification in about three server round trips: when IDLE wakes, the watcher asks its own connection only for UIDs above the newest cached one (`IncomingSession::fetch_new`, one `UID FETCH n:*`), downloads those messages for their preview, announces, and only then runs the full sync. Before, the full sync (SELECT, two SEARCHes, two FETCHes on the shared connection) came first; measured on the owner's Gmail inboxes at 0.3-0.8 s, 2.4-3.6 s when another sync ran. Gmail's own delay before it tells IDLE is outside the app.
- 2026-10-09 - Before each IDLE the watcher sends one NOOP and returns at once when its answer (or any earlier one) reported new or removed mail; IDLE `init` / `done` time out like other commands. Reason: mail that arrived while the connection was not idling was otherwise only noticed at the 5-minute renewal, and a connection that died while idling could hang the watcher.
- 2026-10-09 - Every writer of a folder's cache (`sync_folder`, the watcher's arrivals) holds that folder's lock, so two of them never announce the same message.
- 2026-10-09 - `NewMailItem.preview`: the start of the message text (`mail_render::preview`: quoted lines and signature dropped, invisible preheader padding removed, one line, at most 600 characters). The message is downloaded for it only when a notification template shows `{preview}`, for at most 3 messages and up to 512 KB each, and it is kept, so opening it afterwards is instant. The UI cuts it to the chosen length and ends it with "…". The account address is no longer in the default notification (owner: it says nothing useful); `{account}` stays available.
- 2026-10-09 - Notification settings are `AppSettings.notifications` (`NotificationSettings`), stored as separate rows like the other settings; "notifications" keeps its old on / off meaning. Title and text are templates with `{sender}`, `{address}`, `{subject}`, `{preview}`, `{account}`; the ready-made choices in the settings are just templates, so there is no separate "mode" to keep in step. The core refuses values no notification can show (`AppSettings::check`) and reads anything unreadable as its default. `default_settings` gives the defaults for "Restore the defaults".
- 2026-10-09 - Pictures for notifications are drawn by the page (`design/avatarImage.ts`, a canvas): the sender's logo or initial with the avatar tokens, and the user's own picture cut to a 128 px PNG before it is kept (at most 256 KB, `data:image/png` only, checked again by the core and the shell). Reason: Electron cannot turn SVG logos into a notification icon, and no image library is needed in the core.
- 2026-10-09 - "Stays on screen" for a set time is done by the shell closing the notification itself, with the system's own timeout turned off; "until you close it" is Electron's `timeoutType: 'never'`. Both work on Windows and Linux; macOS decides by itself, which the settings say.
- 2026-10-09 - Settings writes from the UI are queued one after another, and notification choices show before the core confirms them. Reason: the core runs requests side by side, so an earlier write finishing last stored an older value, and template fields lost typed characters.
- 2026-10-09 - A modal window is never taller than the app's window; its content scrolls. Reason: the settings ran off the bottom of the screen once notifications were customised, and their last controls could not be reached.
- 2026-10-09 - The settings window shows one category at a time, picked from a list with icons on its left (vertical tabs: arrow keys, Home / End), instead of one long page. The window keeps one size whatever is open, a long category scrolls on its own, and the category opened last is opened again while the app runs. A new group of settings is a new entry in that list. Reason: owner found the long page hard to find one's way in. Icons `account`, `privacy` and `bell` added for it.
- 2026-10-09 - A short announcement or question is a `MessageBox` (design primitive, laid out like a Windows 7 task dialog: a narrow window, a large glossy picture at the top left (`MessageIcon`, drawn on its own because toolbar icons are too plain at that size), one blue line that says what it is about with the explanation under it, buttons on a grey strip along the bottom; the first button has the focus, Escape gives the safe answer). Adding an account ends in one ("Account added", OK), saying whether the account is open now; when it is opened, the settings window closes first. The removal confirmation uses it too. Reason: owner's request - adding an account ended without a word.
- 2026-10-10 - While the address is typed, the core says what its domain is so far (`recognize_address`, `TypedDomain`): a provider with a mark of its own (`Brand`, only Gmail so far), from its name on (`gmail`, `gmail.`) and the whole domain (`gmail.com`), but no other ending on the way there; a likely typo, which is the provider's name with an ending it does not use (`gmail.ru`), only for providers whose list of domains is complete and whose name nobody else uses (`closed_domains`; not Outlook or Yahoo, which have country domains, nor `mail.ru`); or nothing. A typo is a warning with a "Change it to gmail.com" button, never a block: the user can go on with the address as typed. Reason: owner's request; section 2.2 - the hint suggests, the user decides, and a failed hint leaves the field as it was.
- 2026-10-10 - Provider marks are drawn inside the app (`design/BrandMark.svelte`): the provider's own logo in its own colours, each colour a glossy material (tokens `--brand-*`), on a small paper tile under the avatars' glass, shown at the end of the address field (`TextField`'s `trailing`). Reason: the logo stays recognisable and still looks part of the Aero theme; nothing is downloaded for it.
- 2026-10-10 - An account can be added by picking it on the provider's page instead of typing it ("or using" and a button per provider on the first setup step; `sign_in_choices`, `begin_chosen_sign_in`, then the usual `finish_sign_in` / `cancel_sign_in`). Offered only for a provider with a mark whose page this build can sign in on, so a build without a client id shows no buttons and the address field works as before. The sign-in leaves out `login_hint`, asks with `prompt=select_account consent` so the list shows even for one signed-in account, and adds the provider's identity scope (`openid email` for Google) to learn the address; the engine then fills in the address and both usernames and goes on exactly like a typed sign-in (same checks, same duplicate refusal, which can only happen after the browser here). The display name stays empty, as when it is left blank by hand. Reason: owner's request. Google's consent screen must allow the `openid` and `email` scopes (both non-sensitive).
- 2026-10-10 - `ErrorKind::Duplicate` for adding an account that is already in the app, instead of `Invalid`. Reason: picking an account on Google's page makes it a likely mistake, and "something entered isn't valid" did not say what was wrong.
- 2026-10-10 - The theme is a setting (`AppSettings.theme`: `System`, `Aero`, `Slate`; stored as "system" / "aero" / "slate", default `System`), chosen in a new Appearance category of the settings from previews. The page shows it as `<html data-theme>` (none for `System`, which follows the OS); `design/theme.ts` holds each theme's attribute and whether it is light or dark, as a record over `Theme`, so a theme added to the core cannot be missed there. A new theme is a variant of `Theme`, one `[data-theme]` block in `tokens.css` and an entry in `theme.ts`. Reason: owner's request, with room for more themes.
- 2026-10-10 - Theme blocks in `tokens.css` apply to any element, not only `<html>` (`[data-theme='slate']`, and `[data-theme='aero']` next to `:root`); the OS-dark block applies only while no theme is chosen (`:root:not([data-theme])`). Reason: a preview is drawn with its theme's real tokens whatever the app shows, and a later theme is not overridden by the OS preference.
- 2026-10-10 - The shell follows the theme (`shell:set-color-scheme`: system / light / dark sets Electron's `nativeTheme.themeSource`, so its own dialogs and menus match), and the window stays hidden until the page has first said which theme it shows (at most 3 seconds). Reason: without it a non-default theme showed the OS's colours for a moment at every start.
- 2026-10-10 - A typo warning appears for any ending of a closed provider's name that is not its whole domain, including one still being typed (`gmail.c`), and waits 350 ms before showing, so typing the right ending never flashes it; the mark and the removal of a warning are immediate. Reason: owner's request.
- 2026-10-10 - The address field can hold a provider's domain fixed (`address_presets`, `AddressPreset`: every provider of the built-in table with its first domain): a dropdown at the field's end, "None" first and the default; with a provider picked only the name is typed and the domain shows dimmed after it (`TextField`'s `suffix`), typing or pasting `@` goes back to the whole address, and "None" keeps the address as it was. The arrow and the provider's mark share one place and both open the list. Reason: owner's request, in the way Google's own sign-in page shows the domain.

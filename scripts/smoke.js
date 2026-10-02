// `pnpm test:app`: starts the real shell with the built UI and a throwaway
// data folder, and checks that the page reaches the Rust core.
// Needs a display (on a headless machine: `xvfb-run -a pnpm test:app`).

import assert from 'node:assert/strict'
import { mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { _electron } from 'playwright'
import { run, shellEnv } from './run.js'

run('cargo build -p mail-host')
run('pnpm exec vite build')
run('pnpm exec tsc -p electron')

const dataDirectory = await mkdtemp(path.join(tmpdir(), 'mailclient-smoke-'))
const env = { ...shellEnv(), MAILCLIENT_DATA_DIR: dataDirectory }
delete env.MAILCLIENT_DEV_URL

// A Wayland session without its socket (xvfb-run) has to be told to use X11.
const args = process.env.WAYLAND_DISPLAY === undefined ? ['--ozone-platform=x11', '.'] : ['.']

const app = await _electron.launch({ args, env })
try {
    const page = await app.firstWindow()
    const violations = []
    page.on('console', (message) => {
        if (message.text().includes('Content Security Policy')) {
            violations.push(message.text())
        }
    })

    // No account yet, so the setup form is what a working app shows.
    await page.locator('input[type="email"]').waitFor({ timeout: 15000 })

    const accounts = await page.evaluate(() => window.mailclient.call('list_accounts', {}))
    assert.deepEqual(accounts, { ok: true, value: [] })

    const port = await page.evaluate(() => window.mailclient.call('default_port', { protocol: 'Imap', security: 'Tls' }))
    assert.deepEqual(port, { ok: true, value: 993 })

    const unknown = await page.evaluate(() => window.mailclient.call('no_such_command', {}))
    assert.equal(unknown.ok, false)
    assert.equal(unknown.error.kind, 'Invalid')

    const link = await page.evaluate(() => window.mailclient.openExternal('file:///etc/passwd'))
    assert.equal(link.ok, false)

    // Message HTML is shown in a frame like this one; the page's policy must let it load.
    const framed = await page.evaluate(() => new Promise((resolve) => {
        const frame = document.createElement('iframe')
        frame.sandbox.add('allow-same-origin')
        frame.srcdoc = '<p style="color: red">framed</p>'
        frame.addEventListener('load', () => resolve(frame.contentDocument?.body.textContent ?? ''))
        document.body.append(frame)
    }))
    assert.equal(framed, 'framed')

    assert.deepEqual(violations, [])
    console.log('desktop smoke test passed')
} finally {
    await app.close()
    await rm(dataDirectory, { recursive: true, force: true })
}

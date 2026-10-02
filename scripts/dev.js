// `pnpm dev`: builds the mail core and the shell, then opens the app with the
// UI served by the Vite dev server (hot reload).

import { spawn } from 'node:child_process'
import electron from 'electron'
import { createServer } from 'vite'
import { run, shellEnv } from './run.js'

run('cargo build -p mail-host')
run('pnpm exec tsc -p electron')

const server = await createServer()
await server.listen()

const url = server.resolvedUrls?.local[0]
if (url === undefined) {
    await server.close()
    throw new Error('the dev server did not report its address')
}

const env = { ...shellEnv(), MAILCLIENT_DEV_URL: url }

const shell = spawn(electron, ['.'], { stdio: 'inherit', env })

shell.on('exit', async (code) => {
    await server.close()
    process.exit(code ?? 0)
})

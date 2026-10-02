// `pnpm build`: the installer for this system, written to release/.

import { run } from './run.js'

run('cargo build -p mail-host --release')
run('pnpm exec vite build')
run('pnpm exec tsc -p electron')
run('pnpm exec electron-builder')

// Shared by the scripts in this folder.

import { execSync } from 'node:child_process'

// Editors that are Electron apps themselves set this; it would start plain Node instead of the shell.
export const shellEnv = () => {
    const env = { ...process.env }
    delete env.ELECTRON_RUN_AS_NODE

    return env
}

// Runs one build step and shows its output; a failing step stops the script.
export const run = (command) => {
    console.log('> ' + command)
    execSync(command, { stdio: 'inherit', env: shellEnv() })
}

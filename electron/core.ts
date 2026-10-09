// The Rust mail core runs as a child process (crates/mail-host): one JSON
// request per line goes to its stdin, one JSON response per line comes back.

import { spawn, type ChildProcessByStdio } from 'node:child_process'
import { createInterface } from 'node:readline'
import type { Readable, Writable } from 'node:stream'

// What every call resolves to. `error` is a MailError (or an AddressError)
// exactly as the core produced it, so the UI can word the message itself.
export type Reply =
    | { ok: true, value: unknown }
    | { ok: false, error: unknown }

export type Core = {
    call: (command: string, params: unknown) => Promise<Reply>
    // What the core announces by itself (new mail, ...), exactly as it sent it.
    onEvent: (listener: (event: unknown) => void) => void
    // Starts the core now. It watches the mailboxes for new mail by itself,
    // so it cannot wait for the first call as it otherwise would.
    start: () => void
    stop: () => void
}

type CoreProcess = ChildProcessByStdio<Writable, Readable, null>

const failure = (detail: string): Reply => {
    return { ok: false, error: { kind: 'Protocol', server: null, detail } }
}

// A line of the core is an answer (it has an `id`) or an announcement.
const parseEvent = (line: string): unknown => {
    let parsed: unknown
    try {
        parsed = JSON.parse(line)
    } catch {
        return undefined
    }

    if (typeof parsed !== 'object' || parsed === null || 'id' in parsed || !('event' in parsed)) {
        return undefined
    }

    return parsed.event
}

const parseResponse = (line: string): { id: number, reply: Reply } | null => {
    let parsed: unknown
    try {
        parsed = JSON.parse(line)
    } catch {
        return null
    }

    if (typeof parsed !== 'object' || parsed === null) {
        return null
    }
    if (!('id' in parsed) || typeof parsed.id !== 'number' || !('ok' in parsed)) {
        return null
    }

    if (parsed.ok === true) {
        return { id: parsed.id, reply: { ok: true, value: 'value' in parsed ? parsed.value : null } }
    }

    return { id: parsed.id, reply: { ok: false, error: 'error' in parsed ? parsed.error : null } }
}

export const createCore = (binary: string, args: string[]): Core => {
    let child: CoreProcess | null = null
    let nextId = 1
    const pending = new Map<number, (reply: Reply) => void>()
    const listeners: ((event: unknown) => void)[] = []

    const failPending = (detail: string) => {
        for (const resolve of pending.values()) {
            resolve(failure(detail))
        }
        pending.clear()
    }

    const start = (): CoreProcess => {
        // stderr goes straight to the terminal: the core never prints mail or passwords there.
        const started = spawn(binary, args, { stdio: ['pipe', 'pipe', 'inherit'] })

        const ended = (detail: string) => {
            if (child === started) {
                child = null
                failPending(detail)
            }
        }
        started.on('error', (error) => ended('the mail core could not start: ' + error.message))
        started.on('exit', () => ended('the mail core stopped'))
        // A write to a core that just died fails here; `exit` reports it.
        started.stdin.on('error', () => undefined)

        createInterface({ input: started.stdout }).on('line', (line) => {
            const event = parseEvent(line)
            if (event !== undefined) {
                for (const listener of listeners) {
                    listener(event)
                }

                return
            }

            const response = parseResponse(line)
            if (response === null) {
                return
            }
            const resolve = pending.get(response.id)
            if (resolve !== undefined) {
                pending.delete(response.id)
                resolve(response.reply)
            }
        })

        return started
    }

    return {
        call: (command, params) => new Promise((resolve) => {
            // Started on first use, and again after a crash.
            if (child === null) {
                child = start()
            }

            const id = nextId
            nextId += 1
            pending.set(id, resolve)
            child.stdin.write(JSON.stringify({ id, command, params }) + '\n')
        }),
        onEvent: (listener) => {
            listeners.push(listener)
        },
        start: () => {
            if (child === null) {
                child = start()
            }
        },
        // The core ends by itself when its stdin closes.
        stop: () => {
            if (child !== null) {
                child.stdin.end()
            }
        },
    }
}

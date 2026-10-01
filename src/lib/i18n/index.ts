// All user-facing text goes through `t`. Only English exists for now; adding a
// language is adding a dictionary with the same keys.

import { en } from './en'

export type MessageKey = keyof typeof en

export const t = (key: MessageKey, params: Record<string, string> = {}): string => {
    let message: string = en[key]
    for (const [name, value] of Object.entries(params)) {
        message = message.replaceAll(`{${name}}`, value)
    }

    return message
}

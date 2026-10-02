// The only thing the page can reach outside itself. CommonJS on purpose: a
// sandboxed preload cannot be an ES module. The page-side type of this object
// is `DesktopBridge` in src/lib/api/bridge.ts; keep the two in step.

import { contextBridge, ipcRenderer } from 'electron'

contextBridge.exposeInMainWorld('mailclient', {
    call: (command: string, params: unknown) => ipcRenderer.invoke('core:call', command, params),
    saveAttachment: (messageId: number, index: number, name: string) => {
        return ipcRenderer.invoke('shell:save-attachment', messageId, index, name)
    },
    openExternal: (url: string) => ipcRenderer.invoke('shell:open-external', url),
})

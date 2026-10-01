import { defineConfig } from 'vitest/config'
import { playwright } from '@vitest/browser-playwright'
import adapter from '@sveltejs/adapter-static'
import { sveltekit } from '@sveltejs/kit/vite'

export default defineConfig({
    plugins: [
        sveltekit({
            compilerOptions: {
                // Force runes mode for the project, except for libraries. Can be removed in svelte 6.
                runes: ({ filename }) => filename.split(/[/\\]/).includes('node_modules') ? undefined : true,
            },
            // The app is a client-side SPA inside the Tauri webview: no server at runtime.
            adapter: adapter({ fallback: 'index.html' }),
        }),
    ],
    // Tauri expects a fixed port and prints its own output.
    clearScreen: false,
    server: { port: 5175, strictPort: true },
    test: {
        expect: { requireAssertions: true },
        projects: [
            {
                extends: './vite.config.ts',
                test: {
                    name: 'client',
                    browser: {
                        enabled: true,
                        provider: playwright(),
                        instances: [{ browser: 'chromium', headless: true }],
                    },
                    include: ['src/**/*.svelte.{test,spec}.{js,ts}'],
                },
            },
            {
                extends: './vite.config.ts',
                test: {
                    name: 'unit',
                    environment: 'node',
                    include: ['src/**/*.{test,spec}.{js,ts}'],
                    exclude: ['src/**/*.svelte.{test,spec}.{js,ts}'],
                },
            },
        ],
    },
})

import path from 'node:path'
import js from '@eslint/js'
import stylistic from '@stylistic/eslint-plugin'
import svelte from 'eslint-plugin-svelte'
import { defineConfig, includeIgnoreFile } from 'eslint/config'
import globals from 'globals'
import ts from 'typescript-eslint'

const gitignorePath = path.resolve(import.meta.dirname, '.gitignore')

export default defineConfig(
    includeIgnoreFile(gitignorePath),
    { ignores: ['src/lib/api/generated/'] },
    js.configs.recommended,
    ts.configs.recommended,
    svelte.configs.recommended,
    {
        languageOptions: { globals: { ...globals.browser, ...globals.node } },
        rules: {
            // typescript-eslint strongly recommend that you do not use the no-undef lint rule on TypeScript projects.
            // see: https://typescript-eslint.io/troubleshooting/faqs/eslint/#i-get-errors-from-the-no-undef-rule-about-global-variables-not-being-defined-even-though-there-are-no-typescript-errors
            'no-undef': 'off',
        },
    },
    {
        files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
        languageOptions: {
            parserOptions: {
                projectService: true,
                extraFileExtensions: ['.svelte'],
                parser: ts.parser,
            },
        },
    },
    {
        // Project code style, see AGENTS.md section 7.1.
        plugins: { '@stylistic': stylistic },
        rules: {
            '@stylistic/semi': ['error', 'never'],
            '@stylistic/quotes': ['error', 'single', { avoidEscape: false, allowTemplateLiterals: 'always' }],
            '@stylistic/comma-dangle': ['error', 'always-multiline'],
            '@stylistic/brace-style': ['error', '1tbs', { allowSingleLine: false }],
            curly: ['error', 'all'],
            'prefer-const': 'error',
            '@typescript-eslint/no-explicit-any': 'error',
            '@typescript-eslint/no-non-null-assertion': 'error',
            '@typescript-eslint/ban-ts-comment': 'error',
        },
    },
    {
        files: ['**/*.ts', '**/*.cts', '**/*.js'],
        rules: { '@stylistic/indent': ['error', 4] },
    },
    {
        files: ['**/*.svelte'],
        rules: {
            'svelte/indent': ['error', { indent: 4 }],
            // Svelte decides between let and const in <script> by reactivity, not by reassignment.
            'prefer-const': 'off',
        },
    },
)

<script lang="ts">
    import type { AuthMethod, Protocol, Security, ServerRole, ServerSettings, SettingsIssue } from '$lib/api'
    import SelectField from '$lib/design/SelectField.svelte'
    import TextField from '$lib/design/TextField.svelte'
    import { t } from '$lib/i18n'
    import { authMethods, incomingProtocols, parsePort, securities } from './setup'

    type Props = {
        role: ServerRole
        server: ServerSettings
        issues: SettingsIssue[]
        onchange: (server: ServerSettings) => void
        onprotocol: (protocol: Protocol) => void
        onsecurity: (security: Security) => void
    }

    let { role, server, issues, onchange, onprotocol, onsecurity }: Props = $props()

    // Template literals keep these keys type-checked against the dictionary.
    const protocolOptions = incomingProtocols.map((value) => ({ value, label: t(`protocol.${value}`) }))
    const securityOptions = securities.map((value) => ({ value, label: t(`security.${value}`) }))
    const authOptions = authMethods.map((value) => ({ value, label: t(`auth.${value}`) }))

    const has = (kinds: SettingsIssue['kind'][]): boolean => {
        return issues.some((issue) => issue.server === role && kinds.includes(issue.kind))
    }
</script>

<fieldset>
    <legend>{t(`role.${role}`)}</legend>

    <div class="grid">
        {#if role === 'Incoming'}
            <SelectField
                label={t('setup.protocol')}
                value={server.protocol}
                options={protocolOptions}
                onchange={onprotocol}
            />
        {/if}
        <div class="host">
            <TextField
                label={t('setup.host')}
                value={server.host}
                invalid={has(['EmptyHost', 'InvalidHost'])}
                oninput={(host) => onchange({ ...server, host })}
            />
        </div>
        <TextField
            label={t('setup.port')}
            value={server.port === 0 ? '' : String(server.port)}
            inputmode="numeric"
            invalid={has(['InvalidPort'])}
            oninput={(port) => onchange({ ...server, port: parsePort(port) })}
        />
        <SelectField
            label={t('setup.security')}
            value={server.security}
            options={securityOptions}
            onchange={onsecurity}
        />
        <SelectField
            label={t('setup.auth')}
            value={server.auth}
            options={authOptions}
            onchange={(auth: AuthMethod) => onchange({ ...server, auth })}
        />
        <div class="username">
            <TextField
                label={t('setup.username')}
                value={server.username}
                invalid={has(['EmptyUsername'])}
                oninput={(username) => onchange({ ...server, username })}
            />
        </div>
    </div>
</fieldset>

<style>
    fieldset {
        margin: 0;
        padding: var(--space-3);
        border: 1px solid var(--surface-border);
        border-radius: var(--radius-panel);
        background: var(--surface-sunken);
        min-width: 0;
    }

    legend {
        padding: 0 var(--space-1);
        color: var(--text-muted);
    }

    .grid {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: var(--space-3);
    }

    .host {
        grid-column: span 2;
    }

    .username {
        grid-column: 1 / -1;
    }

    /* Outgoing has no protocol choice: the server field takes the whole first row. */
    .host:first-child {
        grid-column: 1 / -1;
    }

    @media (max-width: 520px) {
        .grid {
            grid-template-columns: minmax(0, 1fr);
        }

        .host,
        .host:first-child {
            grid-column: auto;
        }
    }
</style>

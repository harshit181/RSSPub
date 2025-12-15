<script lang="ts">
    import { api } from "../lib/api";
    import { emailConfig, isAuthenticated, popup } from "../lib/store";

    let smtp_host = "";
    let smtp_port: number | null = null;
    let smtp_password = "";
    let email_address = "";
    let to_email = "";
    let enable_auto_send = false;
    let statusMsg = "";
    let statusColor = "";
    let isSaving = false;

    $: if ($isAuthenticated) {
        loadEmailConfig();
    }

    async function loadEmailConfig() {
        try {
            const config = await api("/email-config");
            if (config) {
                emailConfig.set(config);
                smtp_host = config.smtp_host || "";
                smtp_port = config.smtp_port || null;

                smtp_password = config.smtp_password || "";
                email_address = config.email_address || "";
                to_email = config.to_email || "";
                enable_auto_send = config.enable_auto_send || false;
            }
        } catch (e) {
            console.error("Failed to load email config", e);
        }
    }

    async function saveEmailConfig() {
        isSaving = true;
        statusMsg = "Saving...";
        statusColor = "var(--text-secondary)";

        try {
            await api("/email-config", "POST", {
                smtp_host,
                smtp_port: smtp_port || 0,
                smtp_username: email_address,
                smtp_password,
                email_address,
                to_email,
                enable_auto_send,
            });

            statusMsg = "Saved successfully!";
            statusColor = "var(--accent-green)";
            setTimeout(() => (statusMsg = ""), 3000);

            loadEmailConfig();
        } catch (e: any) {
            statusMsg = "Error: " + e.message;
            statusColor = "var(--danger)";
        } finally {
            isSaving = false;
        }
    }

    async function handleToggleChange() {
        if (!enable_auto_send) {
            await saveEmailConfig();
        }
    }
</script>

<section id="email-config-section" class="card" style="grid-column: 1 / -1">
    <div class="card-header">
        <img src="/icons/mail.svg" alt="Email Icon" width="20" height="20" />
        <h2>Email Configuration</h2>
        <label
            style="margin-left: auto; display: flex; align-items: center; gap: 10px;"
        >
            <span style="font-size: 0.9rem; color: var(--text-secondary);"
                >Enable Auto Send</span
            >
            <label class="switch">
                <input
                    type="checkbox"
                    bind:checked={enable_auto_send}
                    on:change={handleToggleChange}
                />
                <span class="slider round"></span>
            </label>
        </label>
    </div>

    {#if enable_auto_send}
        <form on:submit|preventDefault={saveEmailConfig} id="email-config-form">
            <div
                id="email-inputs-wrapper"
                style="display: grid; grid-template-columns: 1fr 1fr; gap: 15px; margin-bottom: 15px;"
            >
                <input
                    type="text"
                    bind:value={smtp_host}
                    placeholder="SMTP Host (e.g. smtp.gmail.com)"
                    required
                />
                <input
                    type="number"
                    bind:value={smtp_port}
                    placeholder="Port (e.g. 587)"
                    required
                />
                <input
                    type="password"
                    bind:value={smtp_password}
                    placeholder="SMTP Password (leave empty to keep)"
                />
                <input
                    type="email"
                    bind:value={email_address}
                    placeholder="Email Address"
                    required
                />
                <input
                    type="email"
                    bind:value={to_email}
                    placeholder="To Email (Kindle)"
                    required
                />
            </div>
            <button
                type="submit"
                class="add-btn"
                id="save-config-btn"
                style="width: 100%"
                disabled={isSaving}
            >
                Save Config
            </button>
        </form>
    {/if}
    <div id="email-config-status" style="color: {statusColor}">{statusMsg}</div>
</section>

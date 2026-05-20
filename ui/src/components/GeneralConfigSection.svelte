<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "../lib/api";

    type CoverTextColor = "white" | "black";
    type CoverTextPosition = "top-left" | "top-right" | "center" | "bottom-left" | "bottom-right";
    type CoverTextSize = "small" | "medium" | "large";

    let fetchSinceHours = 24;
    let imageTimeoutSeconds = 45;
    let coverTextEnabled = false;
    let coverTextColor: CoverTextColor = "white";
    let coverTextPosition: CoverTextPosition = "bottom-right";
    let coverTextSize: CoverTextSize = "small";
    let loading = false;
    let message = "";

    onMount(async () => {
        await loadConfig();
    });

    async function loadConfig() {
        try {
            loading = true;
            const config = await api("/general-config");
            fetchSinceHours = config.fetch_since_hours;
            imageTimeoutSeconds = config.image_timeout_seconds;
            coverTextEnabled = config.cover_text_enabled ?? false;
            coverTextColor = config.cover_text_color ?? "white";
            coverTextPosition = config.cover_text_position ?? "bottom-right";
            coverTextSize = config.cover_text_size ?? "small";
        } catch (e: any) {
            message = "Failed to load config: " + e.message;
        } finally {
            loading = false;
        }
    }

    async function saveConfig() {
        try {
            loading = true;
            message = "";
            await api("/general-config", "POST", {
                fetch_since_hours: fetchSinceHours,
                image_timeout_seconds: imageTimeoutSeconds,
                cover_text_enabled: coverTextEnabled,
                cover_text_color: coverTextColor,
                cover_text_position: coverTextPosition,
                cover_text_size: coverTextSize,
            });
            message = "Configuration saved successfully.";
        } catch (e: any) {
            message = "Failed to save config: " + e.message;
        } finally {
            loading = false;
        }
    }
</script>

<section class="card">
    <div class="card-header">
        <img
            src="/icons/settings.svg"
            alt="Settings Icon"
            width="20"
            height="20"
        />
        <h2>General Configuration</h2>
    </div>

    <div class="config-grid">
        <div class="form-group">
            <label for="fetch-since">Oldest RSS Article (hours)</label>
            <div class="input-group">
                <input
                    type="number"
                    id="fetch-since"
                    bind:value={fetchSinceHours}
                    min="1"
                />
            </div>
        </div>

        <div class="form-group">
            <label for="image-timeout">Image Processing Timeout (seconds)</label>
            <div class="input-group">
                <input
                    type="number"
                    id="image-timeout"
                    bind:value={imageTimeoutSeconds}
                    min="1"
                />
            </div>
        </div>

        <div class="form-group">
            <label for="cover-text-enabled">Cover Text in Cover Image</label>
            <div class="input-group">
                <input
                    type="checkbox"
                    id="cover-text-enabled"
                    bind:checked={coverTextEnabled}
                />
            </div>
        </div>

        {#if coverTextEnabled}
            <div class="form-group">
                <label for="cover-text-color">Cover Text Color</label>
                <div class="input-group">
                    <select id="cover-text-color" bind:value={coverTextColor}>
                        <option value="white">White</option>
                        <option value="black">Black</option>
                    </select>
                </div>
            </div>

            <div class="form-group">
                <label for="cover-text-position">Cover Text Position</label>
                <div class="input-group">
                    <select id="cover-text-position" bind:value={coverTextPosition}>
                        <option value="top-left">Top left</option>
                        <option value="top-right">Top right</option>
                        <option value="center">Center</option>
                        <option value="bottom-left">Bottom left</option>
                        <option value="bottom-right">Bottom right</option>
                    </select>
                </div>
            </div>

            <div class="form-group">
                <label for="cover-text-size">Cover Text Size</label>
                <div class="input-group">
                    <select id="cover-text-size" bind:value={coverTextSize}>
                        <option value="small">Small</option>
                        <option value="medium">Medium</option>
                        <option value="large">Large</option>
                    </select>
                </div>
            </div>
        {/if}
    </div>

    <div class="config-actions">
        <button on:click={saveConfig} disabled={loading} class="add-btn-modern">
            {loading ? "Saving..." : "Save Configuration"}
        </button>
        {#if message}
            <span class="config-message" class:error={message.includes("Failed")}
                >{message}</span
            >
        {/if}
    </div>
</section>

<script lang="ts">
    import { popup } from "../lib/store";

    function hidePopup() {
        popup.update((p) => ({ ...p, visible: false }));
    }

    function onConfirm() {
        if ($popup.type === "confirm") {
            $popup.onConfirm();
        }
        hidePopup();
    }

    function onCancel() {
        if ($popup.type === "confirm") {
            $popup.onCancel();
        }
        hidePopup();
    }
</script>

{#if $popup.visible}
    <div id="popup-overlay">
        <div class="popup-box">
            <h3 class="popup-title">{$popup.title || "Confirm Action"}</h3>
            <p class="popup-message">{$popup.message}</p>
            <div class="actions">
                {#if $popup.type === "confirm"}
                    <button class="popup-btn secondary" on:click={onCancel}>Cancel</button>
                    <button class="popup-btn primary" on:click={onConfirm}>OK</button>
                {:else}
                    <button
                        class="popup-btn"
                        class:error={$popup.isError}
                        on:click={hidePopup}>OK</button
                    >
                {/if}
            </div>
        </div>
    </div>
{/if}

<style>
    /* Styles are mostly global in app.css but we can scope or reuse them */
    /* Reuse global styles for consistency if they are in app.css, otherwise define here */
    :global(.popup-btn.error) {
        background: var(--danger);
    }
    
    .actions {
        display: flex;
        justify-content: flex-end;
        gap: 10px;
        margin-top: 20px;
    }
    
    .popup-btn.secondary {
        background: #555;
        color: white;
    }

    .popup-btn.primary {
        background: var(--accent-color, #007bff);
        color: white;
    }
</style>

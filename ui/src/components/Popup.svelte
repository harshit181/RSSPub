<script lang="ts">
    import { popup } from "../lib/store";

    function hidePopup() {
        popup.update((p) => ({ ...p, visible: false }));
    }
</script>

{#if $popup.visible}
    <div id="popup-overlay">
        <div class="popup-box">
            <h3 class="popup-title">{$popup.title}</h3>
            <p class="popup-message">{$popup.message}</p>
            <button
                class="popup-btn"
                class:error={$popup.isError}
                on:click={hidePopup}>OK</button
            >
        </div>
    </div>
{/if}

<style>
    /* Styles are mostly global in app.css but we can scope or reuse them */
    /* Reuse global styles for consistency if they are in app.css, otherwise define here */
    :global(.popup-btn.error) {
        background: var(--danger);
    }
</style>

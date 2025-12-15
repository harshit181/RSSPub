<script lang="ts">
    export let tabs: string[] = [];
    export let activeTab: string = "";

    function selectTab(tab: string) {
        activeTab = tab;
    }
</script>

<div class="tabs">
    {#each tabs as tab}
        <button
            class="tab-btn {activeTab === tab ? 'active' : ''}"
            on:click={() => selectTab(tab)}
        >
            {tab}
        </button>
    {/each}
</div>

<style>
    .tabs {
        display: flex;
        gap: 1rem;
        margin-bottom: 2rem;
        border-bottom: 1px solid var(--card-border);
        padding-bottom: 1px;
    }

    .tab-btn {
        background: transparent;
        color: var(--text-secondary);
        padding: 0.75rem 1.5rem;
        border: none;
        border-bottom: 2px solid transparent;
        cursor: pointer;
        font-size: 1rem;
        font-weight: 500;
        transition: all 0.2s;
        border-radius: 8px 8px 0 0;
    }

    .tab-btn:hover {
        color: var(--text-primary);
        background: rgba(255, 255, 255, 0.03);
    }

    .tab-btn.active {
        color: var(--accent-blue);
        border-bottom-color: var(--accent-blue);
        background: rgba(59, 130, 246, 0.1);
    }
</style>

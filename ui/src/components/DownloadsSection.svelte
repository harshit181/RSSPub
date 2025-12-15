<script lang="ts">
    import { api } from "../lib/api";
    import { downloads, isAuthenticated } from "../lib/store";

    $: if ($isAuthenticated) {
        loadDownloads();
    }

    async function loadDownloads() {
        try {
            const data = await api("/downloads");
            if (data) downloads.set(data);
        } catch (e) {
            console.error(e);
        }
    }
</script>

<section id="downloads-section" class="card">
    <div class="card-header">
        <img
            src="/icons/download.svg"
            alt="Download Icon"
            width="20"
            height="20"
        />
        <h2>Downloads</h2>
    </div>
    <ul id="downloads-list" class="item-list">
        {#each $downloads as file}
            <li>
                <a href="/epubs/{file}" download>{file}</a>
            </li>
        {/each}
    </ul>
</section>

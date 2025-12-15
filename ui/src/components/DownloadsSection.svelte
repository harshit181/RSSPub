<script lang="ts">
    import { api } from '../lib/api';
    import { downloads, isAuthenticated } from '../lib/store';

    $: if ($isAuthenticated) {
        loadDownloads();
    }

    async function loadDownloads() {
        try {
            const data = await api('/downloads');
            if (data) downloads.set(data);
        } catch (e) {
            console.error(e);
        }
    }
</script>

<section id="downloads-section" class="card">
    <div class="card-header">
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="7 10 12 15 17 10"></polyline>
            <line x1="12" y1="15" x2="12" y2="3"></line>
        </svg>
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

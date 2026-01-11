<script lang="ts">
    import { get } from "svelte/store";
    import { api } from "../lib/api";
    import { authHeader, downloads, isAuthenticated } from "../lib/store";

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

    async function downloadFile(filename: string) {
        try {
            const headers: Record<string, string> = {};
            const auth = get(authHeader);
            if (auth) {
                headers["Authorization"] = auth;
            }

            const response = await fetch(`/epubs/${filename}`, { headers });
            
            if (response.status === 401) {
                window.dispatchEvent(new CustomEvent("unauthorized"));
                return;
            }
            
            if (!response.ok) {
                console.error("Download failed:", response.statusText);
                return;
            }

            const blob = await response.blob();
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = filename;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
        } catch (e) {
            console.error("Download error:", e);
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
                <button on:click={() => downloadFile(file)}>{file}</button>
            </li>
        {/each}
    </ul>
</section>

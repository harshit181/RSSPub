<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "../lib/api";
    import { feeds, isAuthenticated, popup } from "../lib/store";

    let url = "";
    let name = "";
    let concurrency_limit: number | null = null;
    let importStatus = "Import OPML";
    let isImporting = false;
    let fileInput: HTMLInputElement;

    $: if ($isAuthenticated) {
        loadFeeds();
    }

    async function loadFeeds() {
        try {
            const data = await api("/feeds");
            if (data) feeds.set(data);
        } catch (e) {
            console.error(e);
        }
    }

    async function addFeed() {
        if (!url) return;
        try {
            await api("/feeds", "POST", {
                url,
                name: name || null,
                concurrency_limit: concurrency_limit || 0,
            });
            url = "";
            name = "";
            concurrency_limit = null;
            loadFeeds();
        } catch (e: any) {
            popup.set({
                visible: true,
                title: "Error",
                message: e.message,
                isError: true,
            });
        }
    }

    async function deleteFeed(id: number) {
        if (!confirm("Delete this feed?")) return;
        try {
            await api(`/feeds/${id}`, "DELETE");
            loadFeeds();
        } catch (e: any) {
            popup.set({
                visible: true,
                title: "Error",
                message: e.message,
                isError: true,
            });
        }
    }

    async function importOpml(event: Event) {
        const input = event.target as HTMLInputElement;
        if (!input.files || input.files.length === 0) return;

        const file = input.files[0];
        const formData = new FormData();
        formData.append("file", file);

        isImporting = true;
        importStatus = "Importing...";

        try {
            const headers: Record<string, string> = {};

            const auth = localStorage.getItem("rsspub_auth");
            if (auth) headers["Authorization"] = auth;

            const res = await fetch("/feeds/import", {
                method: "POST",
                headers,
                body: formData,
            });

            if (res.ok) {
                popup.set({
                    visible: true,
                    title: "Success",
                    message: "Feeds imported successfully!",
                    isError: false,
                });
                loadFeeds();
            } else {
                throw new Error(await res.text());
            }
        } catch (e: any) {
            popup.set({
                visible: true,
                title: "Import Failed",
                message: e.message || "Unknown error",
                isError: true,
            });
        } finally {
            isImporting = false;
            importStatus = "Import OPML";
            input.value = "";
        }
    }
</script>

<section id="feeds-section" class="card">
    <div class="card-header">
        <img src="/icons/rss.svg" alt="Feed Icon" width="20" height="20" />
        <h2>Feeds</h2>
        <input
            type="file"
            bind:this={fileInput}
            id="opml-file"
            accept=".opml,.xml"
            style="display: none"
            on:change={importOpml}
        />
        <button
            on:click={() => fileInput.click()}
            class="add-btn"
            style="margin-left: auto; padding: 5px 10px; font-size: 0.8rem;"
            disabled={isImporting}
        >
            {importStatus}
        </button>
    </div>

    <ul id="feeds-list" class="item-list">
        {#each $feeds as feed (feed.id)}
            <li>
                <div style="display: flex; align-items: center; gap: 10px;">
                    <img
                        src="/icons/rss.svg"
                        alt="Feed Icon"
                        width="18"
                        height="18"
                        class="rss-icon"
                        style="filter: invert(36%) sepia(74%) saturate(836%) hue-rotate(185deg) brightness(97%) contrast(92%); flex-shrink: 0;"
                    />
                    <span>
                        {feed.name || feed.url}
                        <small
                            >({feed.concurrency_limit === 0
                                ? "Unlimited"
                                : feed.concurrency_limit + " threads"})</small
                        >
                    </span>
                </div>
                <button on:click={() => deleteFeed(feed.id)} class="delete-btn"
                    >×</button
                >
            </li>
        {/each}
    </ul>

    <form on:submit|preventDefault={addFeed} id="add-feed-form">
        <div class="input-group">
            <input
                type="url"
                bind:value={url}
                placeholder="Feed URL"
                required
            />
            <input
                type="text"
                bind:value={name}
                placeholder="Name (Optional)"
            />
        </div>
        <div class="input-group">
            <input
                type="number"
                bind:value={concurrency_limit}
                placeholder="Limit (0=Uni)"
                min="0"
            />
            <button type="submit" class="add-btn"> Add Feed </button>
        </div>
    </form>
</section>

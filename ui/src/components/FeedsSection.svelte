<script lang="ts">
    import { api } from "../lib/api";
    import { feeds, isAuthenticated, popup } from "../lib/store";
    import yaml from "js-yaml";

    let url = "";
    let name = "";
    let concurrency_limit: number | null = null;
    let processor = "default";  
    let customConfig = "";
    let customConfigError = "";
    let importStatus = "Import OPML";
    let isImporting = false;
    let fileInput: HTMLInputElement;
    
    let editModalOpen = false;
    let editingFeedId: number | null = null;
    let editingFeedName = "";
    let editProcessor = "default";
    let editCustomConfig = "";
    let editCustomConfigError = "";

    let draggedIndex: number | null = null;
    let dropTargetIndex: number | null = null;

    function validateYaml(value: string): string {
        if (!value.trim()) {
            return "Custom config cannot be empty";
        }
        try {
            const parsed = yaml.load(value);
            if (typeof parsed !== 'object' || parsed === null) {
                return "Config must be a YAML object";
            }
            if (!('selector' in parsed) || !Array.isArray((parsed as any).selector)) {
                return "Config must have 'selector' as an array";
            }
            return "";
        } catch (e: any) {
            return `Invalid YAML: ${e.message}`;
        }
    }

    $: if (processor === "custom") {
        customConfigError = validateYaml(customConfig);
    } else {
        customConfigError = "";
    }

    $: if (editProcessor === "custom") {
        editCustomConfigError = validateYaml(editCustomConfig);
    } else {
        editCustomConfigError = "";
    }

    $: isAddFormValid = processor !== "custom" || !customConfigError;

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
        if (processor === "custom" && customConfigError) {
            popup.set({
                visible: true,
                title: "Validation Error",
                message: customConfigError,
                isError: true,
            });
            return;
        }
        try {
            await api("/feeds", "POST", {
                url,
                name: name || null,
                concurrency_limit: concurrency_limit || 0,
                processor: processor,
                custom_config: processor === "custom" ? customConfig : null,
            });
            url = "";
            name = "";
            concurrency_limit = null;
            processor = "default";
            customConfig = "";
            customConfigError = "";
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

    function deleteFeed(id: number) {
        popup.set({
            visible: true,
            title: "Confirm Deletion",
            message: "Delete this feed?",
            isError: false,
            type: "confirm",
            onConfirm: async () => {
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
            },
            onCancel: () => {},
        });
    }

    let editingUrl = "";
    let editingConcurrencyLimit: number | null = null;

    function openEditFeed(feed: any) {
        editingFeedId = feed.id;
        editingFeedName = feed.name || "";
        editingUrl = feed.url;
        editingConcurrencyLimit = feed.concurrency_limit;
        
        if (feed.feed_processor) {
            editProcessor = feed.feed_processor.processor || "default";
            editCustomConfig = feed.feed_processor.custom_config || "";
        } else {
            editProcessor = "default";
            editCustomConfig = "";
        }
        
        editModalOpen = true;
    }

    async function saveFeed() {
        if (!editingFeedId) return;
        if (editProcessor === "custom" && editCustomConfigError) {
            popup.set({
                visible: true,
                title: "Validation Error",
                message: editCustomConfigError,
                isError: true,
            });
            return;
        }
        
        try {
            // Update feed details (including processor)
            await api(`/feeds/${editingFeedId}`, "PUT", {
                url: editingUrl,
                name: editingFeedName || null,
                concurrency_limit: editingConcurrencyLimit || 0,
                processor: editProcessor,
                custom_config: editProcessor === "custom" ? editCustomConfig : null,
            });

            editModalOpen = false;
            editingFeedId = null;
            editCustomConfigError = "";
            loadFeeds();
            popup.set({
                visible: true,
                title: "Success",
                message: "Feed updated!",
                isError: false,
            });
        } catch (e: any) {
            popup.set({
                visible: true,
                title: "Error",
                message: e.message,
                isError: true,
            });
        }
    }

    function closeEditModal() {
        editModalOpen = false;
        editingFeedId = null;
        editingUrl = "";
        editingConcurrencyLimit = null;
        editProcessor = "default";
        editCustomConfig = "";
        editCustomConfigError = "";
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

    function handleDragStart(event: DragEvent, index: number) {
        draggedIndex = index;
        if (event.dataTransfer) {
            event.dataTransfer.effectAllowed = "move";
            event.dataTransfer.setData("text/plain", index.toString());
        }
    }

    function handleDragOver(event: DragEvent, index: number) {
        event.preventDefault();
        if (event.dataTransfer) {
            event.dataTransfer.dropEffect = "move";
        }
        dropTargetIndex = index;
    }

    function handleDragLeave() {
        dropTargetIndex = null;
    }

    function handleDragEnd() {
        draggedIndex = null;
        dropTargetIndex = null;
    }

    async function handleDrop(event: DragEvent, targetIndex: number) {
        event.preventDefault();
        
        if (draggedIndex === null || draggedIndex === targetIndex) {
            draggedIndex = null;
            dropTargetIndex = null;
            return;
        }

        const currentFeeds = [...$feeds];
        const [draggedFeed] = currentFeeds.splice(draggedIndex, 1);
        currentFeeds.splice(targetIndex, 0, draggedFeed);

        feeds.set(currentFeeds);

        const feedPositions = currentFeeds.map((feed, index) => ({
            id: feed.id,
            position: index,
        }));

        try {
            await api("/feeds/reorder", "POST", { feeds: feedPositions });
        } catch (e: any) {
            popup.set({
                visible: true,
                title: "Error",
                message: "Failed to save feed order: " + e.message,
                isError: true,
            });
            loadFeeds();
        }

        draggedIndex = null;
        dropTargetIndex = null;
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
        {#each $feeds as feed, index (feed.id)}
            <li
                draggable="true"
                on:dragstart={(e) => handleDragStart(e, index)}
                on:dragover={(e) => handleDragOver(e, index)}
                on:dragleave={handleDragLeave}
                on:dragend={handleDragEnd}
                on:drop={(e) => handleDrop(e, index)}
                class:dragging={draggedIndex === index}
                class:drop-target={dropTargetIndex === index && draggedIndex !== index}
            >
                <div style="display: flex; align-items: center; gap: 10px; flex: 1;">
                    <span class="drag-handle" title="Drag to reorder">⋮⋮</span>
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
                <div style="display: flex; gap: 8px; align-items: center;">
                    <button 
                        on:click={() => openEditFeed(feed)} 
                        class="edit-btn"
                        title="Edit feed"
                    >✎</button>
                    <button on:click={() => deleteFeed(feed.id)} class="delete-btn">×</button>
                </div>
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
            <select bind:value={processor}>
                <option value="default">Default</option>
                <option value="dom_smoothie">DomSmoothie</option>
                <option value="text_only">Text Only (No Images)</option>
                <option value="custom">Custom (Experimental)</option>
            </select>
            <button type="submit" class="add-btn" disabled={!isAddFormValid}> Add Feed </button>
        </div>
        {#if processor === "custom"}
            <div class="input-group" style="margin-top: 10px;">
                <textarea
                    bind:value={customConfig}
                    placeholder="selector:
  - '.article-content'
discard:
  - '.ads'
output_mode: html"
                    rows="5"
                    style="width: 100%; font-family: monospace; font-size: 0.85rem;"
                    class:invalid={customConfigError}
                ></textarea>
            </div>
            {#if customConfigError}
                <div class="validation-error">{customConfigError}</div>
            {/if}
        {/if}
    </form>
</section>

<!-- Edit Feed Modal -->
{#if editModalOpen}
    <div class="modal-overlay" on:click={closeEditModal}>
        <div class="modal-box" on:click|stopPropagation>
            <h3>Edit Feed</h3>
            
            <div class="modal-field">
                <label>URL</label>
                <input type="url" bind:value={editingUrl} placeholder="Feed URL" />
            </div>

            <div class="modal-field">
                <label>Name</label>
                <input type="text" bind:value={editingFeedName} placeholder="Name (Optional)" />
            </div>

            <div class="modal-field">
                <label>Concurrency Limit (0 = Unlimited)</label>
                <input type="number" bind:value={editingConcurrencyLimit} min="0" />
            </div>

            <hr style="margin: 15px 0; border: 0; border-top: 1px solid #444;" />
            
            <div class="modal-field">
                <label>Processor Type</label>
                <select bind:value={editProcessor}>
                    <option value="default">Default</option>
                    <option value="dom_smoothie">DomSmoothie</option>
                    <option value="text_only">Text Only (No Images)</option>
                    <option value="custom">Custom (Experimental)</option>
                </select>
            </div>
            
            {#if editProcessor === "custom"}
                <div class="modal-field">
                    <label>Custom Config (YAML)</label>
                    <textarea
                        bind:value={editCustomConfig}
                        placeholder="selector:
  - '.article-content'
discard:
  - '.ads'
output_mode: html"
                        rows="8"
                        style="font-family: monospace; font-size: 0.85rem;"
                        class:invalid={editCustomConfigError}
                    ></textarea>
                    {#if editCustomConfigError}
                        <div class="validation-error">{editCustomConfigError}</div>
                    {/if}
                </div>
            {/if}
            
            <div class="modal-actions">
                <button class="cancel-btn" on:click={closeEditModal}>Cancel</button>
                <button class="add-btn" on:click={saveFeed} disabled={editProcessor === "custom" && !!editCustomConfigError}>Save</button>
            </div>
        </div>
    </div>
{/if}

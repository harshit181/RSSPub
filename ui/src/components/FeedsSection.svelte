<script lang="ts">
    import { api } from "../lib/api";
    import { feeds, categories, isAuthenticated, popup } from "../lib/store";
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
    let draggedFeedId: number | null = null;
    let dropTargetIndex: number | null = null;
    let dropTargetCategoryId: number | null = null;

    let draggedCategoryIndex: number | null = null;
    let dropTargetCategoryIndex: number | null = null;

    let newCategoryName = "";

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
        loadData();
    }

    async function loadData() {
        await Promise.all([loadCategories(), loadFeeds()]);
    }

    async function loadCategories() {
        try {
            const data = await api("/categories");
            if (data) categories.set(data);
        } catch (e) {
            console.error(e);
        }
    }

    async function loadFeeds() {
        try {
            const data = await api("/feeds");
            if (data) feeds.set(data);
        } catch (e) {
            console.error(e);
        }
    }

    async function addCategory() {
        if (!newCategoryName) return;
        try {
            await api("/categories", "POST", { name: newCategoryName });
            newCategoryName = "";
            loadCategories();
        } catch (e: any) {
            popup.set({ visible: true, title: "Error", message: e.message, isError: true });
        }
    }

    function deleteCategory(id: number) {
        popup.set({
            visible: true, title: "Confirm", message: "Delete this category? Feeds inside will become uncategorized.",
            isError: false, type: "confirm",
            onConfirm: async () => {
                try {
                    await api(`/categories/${id}`, "DELETE");
                    loadCategories();
                    loadFeeds();
                } catch (e: any) {
                    popup.set({ visible: true, title: "Error", message: e.message, isError: true });
                }
            }, onCancel: () => {},
        });
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

    function handleDragStart(event: DragEvent, feedId: number, index: number) {
        draggedIndex = index;
        draggedFeedId = feedId;
        event.stopPropagation();
        if (event.dataTransfer) {
            event.dataTransfer.effectAllowed = "move";
            event.dataTransfer.setData("text/plain", "feed:" + feedId);
        }
    }

    function handleDragOver(event: DragEvent, index: number, categoryId: number | null) {
        event.preventDefault();
        event.stopPropagation();
        if (event.dataTransfer) {
            event.dataTransfer.dropEffect = "move";
        }
        dropTargetIndex = index;
        dropTargetCategoryId = categoryId;
    }

    function handleDragLeave(event: DragEvent) {
        event.stopPropagation();
        dropTargetIndex = null;
        dropTargetCategoryId = null;
    }

    function handleDragEnd() {
        draggedIndex = null;
        draggedFeedId = null;
        dropTargetIndex = null;
        dropTargetCategoryId = null;
        draggedCategoryIndex = null;
        dropTargetCategoryIndex = null;
    }

    async function handleDrop(event: DragEvent, targetIndex: number, targetCategoryId: number | null) {
        event.preventDefault();
        event.stopPropagation();
        
        const data = event.dataTransfer?.getData("text/plain");
        if (!data || !data.startsWith("feed:")) return handleDragEnd();

        const feedId = parseInt(data.split(":")[1]);
        const feed = $feeds.find(f => f.id === feedId);
        if (!feed) return handleDragEnd();

        // Check if category changed
        const currentCategoryId = feed.category_id || null;
        if (currentCategoryId !== targetCategoryId) {
            // Update feed's category assignment (we call PUT feed to update it entirely)
            try {
                await api(`/feeds/${feed.id}`, "PUT", {
                    url: feed.url,
                    name: feed.name,
                    concurrency_limit: feed.concurrency_limit,
                    processor: feed.feed_processor?.processor || "default",
                    custom_config: feed.feed_processor?.custom_config || null,
                    category: { id: targetCategoryId }
                });
            } catch (e: any) {
                console.error(e);
            }
        }

        const currentFeeds = [...$feeds];
        const draggedFeedIndex = currentFeeds.findIndex(f => f.id === feedId);
        if (draggedFeedIndex !== -1 && draggedFeedIndex !== targetIndex) {
            const [draggedFeed] = currentFeeds.splice(draggedFeedIndex, 1);
            currentFeeds.splice(targetIndex, 0, draggedFeed);
            feeds.set(currentFeeds);

            const feedPositions = currentFeeds.map((f, i) => ({ id: f.id, position: i }));
            try {
                await api("/feeds/reorder", "POST", { feeds: feedPositions });
            } catch (e: any) {
                console.error(e);
            }
        }
        loadData();
        handleDragEnd();
    }

    function handleCategoryDragStart(event: DragEvent, index: number) {
        draggedCategoryIndex = index;
        if (event.dataTransfer) {
            event.dataTransfer.effectAllowed = "move";
            event.dataTransfer.setData("text/plain", "category:" + index);
        }
    }

    function handleCategoryDragOver(event: DragEvent, index: number) {
        event.preventDefault();
        if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
        dropTargetCategoryIndex = index;
    }

    async function handleCategoryDrop(event: DragEvent, targetIndex: number) {
        event.preventDefault();
        const data = event.dataTransfer?.getData("text/plain");
        if (!data || !data.startsWith("category:")) return handleDragEnd();

        if (draggedCategoryIndex === null || draggedCategoryIndex === targetIndex) return handleDragEnd();

        const currentCategories = [...$categories];
        const [draggedCat] = currentCategories.splice(draggedCategoryIndex, 1);
        currentCategories.splice(targetIndex, 0, draggedCat);
        categories.set(currentCategories);

        const pos = currentCategories.map((c, i) => ({ id: c.id, position: i }));
        try {
            await api("/categories/reorder", "POST", { categories: pos });
        } catch (e: any) {
            console.error(e);
        }
        handleDragEnd();
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
            class="add-btn import-btn"
            disabled={isImporting}
        >
            {importStatus}
        </button>
    </div>

    <!-- Add Category Form -->
    <form on:submit|preventDefault={addCategory} class="category-add-form">
        <input type="text" bind:value={newCategoryName} placeholder="New Category Name" required class="category-input" />
        <button type="submit" class="add-btn-modern">Add Category</button>
    </form>

    <!-- Uncategorized Feeds -->
    <div class="category-group uncategorized-group"
         on:dragover={(e) => handleDragOver(e, $feeds.length, null)} 
         on:drop={(e) => handleDrop(e, $feeds.length, null)}>
        <h3 class="uncategorized-title">Uncategorized</h3>
        <ul class="item-list">
            {#each $feeds.map((f, i) => ({f, i})).filter(x => !x.f.category_id) as item (item.f.id)}
                <li
                    draggable="true"
                    on:dragstart={(e) => handleDragStart(e, item.f.id, item.i)}
                    on:dragover={(e) => handleDragOver(e, item.i, null)}
                    on:dragleave={handleDragLeave}
                    on:dragend={handleDragEnd}
                    on:drop={(e) => handleDrop(e, item.i, null)}
                    class:dragging={draggedFeedId === item.f.id}
                    class:drop-target={dropTargetIndex === item.i && dropTargetCategoryId === null}
                >
                    <div class="feed-item-info">
                        <span class="drag-handle" title="Drag to reorder">⋮⋮</span>
                        <img src="/icons/rss.svg" alt="Feed Icon" width="18" height="18" class="feed-icon" />
                        <span>{item.f.name || item.f.url} <small>({item.f.concurrency_limit === 0 ? "Unlimited" : item.f.concurrency_limit + " threads"})</small></span>
                    </div>
                    <div class="feed-item-actions">
                        <button on:click={() => openEditFeed(item.f)} class="edit-btn">✎</button>
                        <button on:click={() => deleteFeed(item.f.id)} class="delete-btn">×</button>
                    </div>
                </li>
            {/each}
        </ul>
    </div>

    <!-- Category Groups -->
    {#each $categories as cat, catIndex (cat.id)}
        <div class="category-group styled-category-group"
             draggable="true"
             on:dragstart={(e) => handleCategoryDragStart(e, catIndex)}
             on:dragover={(e) => handleCategoryDragOver(e, catIndex)}
             on:drop={(e) => handleCategoryDrop(e, catIndex)}
             class:drop-target={dropTargetCategoryIndex === catIndex && draggedCategoryIndex !== catIndex}>
            <div class="category-header">
                <h3 class="category-title">
                    <span class="drag-handle category-drag">⋮⋮</span> {cat.name}
                </h3>
                <button on:click={() => deleteCategory(cat.id)} class="delete-btn category-delete-btn">Delete Category</button>
            </div>
            
            <ul class="item-list category-item-list"
                on:dragover|stopPropagation={(e) => handleDragOver(e, $feeds.length, cat.id)}
                on:drop|stopPropagation={(e) => handleDrop(e, $feeds.length, cat.id)}>
                {#each $feeds.map((f, i) => ({f, i})).filter(x => x.f.category_id === cat.id) as item (item.f.id)}
                    <li
                        draggable="true"
                        on:dragstart|stopPropagation={(e) => handleDragStart(e, item.f.id, item.i)}
                        on:dragover|stopPropagation={(e) => handleDragOver(e, item.i, cat.id)}
                        on:drop|stopPropagation={(e) => handleDrop(e, item.i, cat.id)}
                        class:dragging={draggedFeedId === item.f.id}
                        class:drop-target={dropTargetIndex === item.i && dropTargetCategoryId === cat.id}
                    >
                        <div style="display: flex; align-items: center; gap: 10px; flex: 1;">
                            <span class="drag-handle" title="Drag to reorder">⋮⋮</span>
                            <img src="/icons/rss.svg" alt="Feed Icon" width="18" height="18" style="filter: invert(36%) sepia(74%) saturate(836%) hue-rotate(185deg) brightness(97%) contrast(92%);" />
                            <span>{item.f.name || item.f.url} <small>({item.f.concurrency_limit === 0 ? "Unlimited" : item.f.concurrency_limit + " threads"})</small></span>
                        </div>
                        <div style="display: flex; gap: 8px; align-items: center;">
                            <button on:click={() => openEditFeed(item.f)} class="edit-btn">✎</button>
                            <button on:click={() => deleteFeed(item.f.id)} class="delete-btn">×</button>
                        </div>
                    </li>
                {/each}
            </ul>
        </div>
    {/each}

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

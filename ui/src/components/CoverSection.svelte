<script lang="ts">
    import { isAuthenticated } from "../lib/store";

    let coverSrc = "/cover.jpg";
    let isUploading = false;
    let uploadStatus = "";
    let statusColor = "";
    let fileInput: HTMLInputElement;

    async function uploadCover(event: Event) {
        if (!fileInput.files || fileInput.files.length === 0) return;

        const file = fileInput.files[0];
        const formData = new FormData();
        formData.append("cover", file);

        isUploading = true;
        uploadStatus = "Uploading...";
        statusColor = "var(--text-secondary)";

        try {
            const headers: Record<string, string> = {};
            const auth = localStorage.getItem("rsspub_auth");
            if (auth) headers["Authorization"] = auth;

            const res = await fetch("/cover", {
                method: "POST",
                headers,
                body: formData,
            });

            if (res.ok) {
                uploadStatus = "Cover updated successfully!";
                statusColor = "var(--accent-green)";
                coverSrc = "/cover.jpg?t=" + new Date().getTime();
                fileInput.value = "";
            } else {
                throw new Error(await res.text());
            }
        } catch (e: any) {
            uploadStatus = "Error: " + e.message;
            statusColor = "var(--danger)";
        } finally {
            isUploading = false;
        }
    }
</script>

<section id="cover-section" class="card">
    <div class="card-header">
        <h2>Cover Image</h2>
    </div>
    <div class="cover-wrapper">
        <div class="current-cover">
            <div class="cover-overlay">
                <span>DAILY RSS DIGEST<br />(Cover Preview)</span>
            </div>
            <img id="cover-preview" src={coverSrc} alt="Book Cover" />
        </div>
        <div class="upload-cover">
            <label for="cover-file" class="upload-label">Replace Cover</label>
            <form on:submit|preventDefault={uploadCover} id="upload-cover-form">
                <div class="file-input-wrapper">
                    <input
                        type="file"
                        id="cover-file"
                        name="cover"
                        accept="image/jpeg"
                        required
                        bind:this={fileInput}
                    />
                </div>
                <button type="submit" class="add-btn" disabled={isUploading}>
                    Upload
                </button>
            </form>
            <div id="upload-status" style="color: {statusColor}">
                {uploadStatus}
            </div>
        </div>
    </div>
</section>

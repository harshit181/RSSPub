<script lang="ts">
    import { api } from "../lib/api";
    import { downloads, emailConfig, isAuthenticated } from "../lib/store";

    let isGenerating = false;
    let status = "";

    async function generate() {
        if (isGenerating) return;
        isGenerating = true;
        status = "Requesting generation...";

        try {
            let send_email = false;

            emailConfig.subscribe((c) => {
                if (c) send_email = c.enable_auto_send;
            })();

            const res = await api("/generate", "POST", {
                feeds: [],
                send_email,
            });

            status = "Generation started in background. Please wait...";

            let checks = 0;
            const interval = setInterval(async () => {
                checks++;
                try {
                    const data = await api("/downloads");
                    if (data) downloads.set(data);
                } catch (e) {}

                if (checks > 10) {
                    clearInterval(interval);
                    status = "Generation started. Check downloads list below.";
                    isGenerating = false;
                }
            }, 2000);
        } catch (e: any) {
            status = "Error: " + e.message;
            isGenerating = false;
        }
    }
</script>

<section id="generate-section" class="card">
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
            <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
        </svg>
        <h2>Manual Generation</h2>
    </div>
    <div class="generate-wrapper">
        <button id="generate-btn" on:click={generate} disabled={isGenerating}
            >Generate EPUB Now</button
        >
        <div id="status">{status}</div>
    </div>
</section>

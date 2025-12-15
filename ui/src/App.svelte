<script lang="ts">
    import Header from "./components/Header.svelte";
    import LoginOverlay from "./components/LoginOverlay.svelte";
    import Popup from "./components/Popup.svelte";
    import FeedsSection from "./components/FeedsSection.svelte";
    import GenerationSection from "./components/GenerationSection.svelte";
    import CoverSection from "./components/CoverSection.svelte";
    import SchedulesSection from "./components/SchedulesSection.svelte";
    import DownloadsSection from "./components/DownloadsSection.svelte";
    import EmailConfigSection from "./components/EmailConfigSection.svelte";
    import { onMount } from "svelte";
    import { api } from "./lib/api";
    import { isAuthenticated } from "./lib/store";

    onMount(async () => {
        try {
            await api("/auth/check");
            isAuthenticated.set(true);
        } catch (e) {
            isAuthenticated.set(false);
        }
    });
</script>

<LoginOverlay />
<Popup />

{#if $isAuthenticated}
    <div class="container">
        <Header />

        <main class="dashboard-grid">
            <div class="column left-col">
                <FeedsSection />
            </div>

            <div class="column right-col">
                <GenerationSection />

                <div class="sub-grid">
                    <CoverSection />

                    <div class="column">
                        <SchedulesSection />
                        <DownloadsSection />
                    </div>

                    <EmailConfigSection />
                </div>
            </div>
        </main>
    </div>
{:else}
    <!--
    If not authenticated, show blank or blur content?
    Original code hid container and showed login overlay.
    Here LoginOverlay handles its own visibility but also depends on auth logic.
    If not authenticated, we simply don't render the dashboard.
    LoginOverlay will appear if api calls return 401, OR we can force it.
    Use onMount to check auth.
-->
{/if}

<style>
    /* App specific styles if any, otherwise global in app.css */
</style>

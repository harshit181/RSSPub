<script lang="ts">
    import { isAuthenticated } from '../lib/store';
</script>

<header>
    <div class="header-content">
        <h1>RSSPub</h1>
        <p>RSS Aggregator & EPUB Generator</p>
    </div>
    <div class="status-indicator">
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            style="color: var(--accent-blue); margin-right: 6px"
        >
            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path>
            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path>
        </svg>
        <span id="connection-status">{$isAuthenticated ? 'Connected' : 'Disconnected'}</span>
    </div>
</header>

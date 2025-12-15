<script lang="ts">
    import { isAuthenticated } from '../lib/store';
</script>

<header>
    <div class="header-content">
        <h1>RSSPub</h1>
        <p>RSS Aggregator & EPUB Generator</p>
    </div>
    <div class="status-indicator">
        <img
            src="/icons/book.svg"
            alt="Logo"
            width="18"
            height="18"
            style="margin-right: 6px; filter: invert(36%) sepia(74%) saturate(836%) hue-rotate(185deg) brightness(97%) contrast(92%);"
        />
        <span id="connection-status">{$isAuthenticated ? 'Connected' : 'Disconnected'}</span>
    </div>
</header>

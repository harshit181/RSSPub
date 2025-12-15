<script lang="ts">
    import { authHeader, isAuthenticated, isLoginVisible } from '../lib/store';

    let username = '';
    let password = '';
    let error = '';

    async function handleLogin() {
        if (!username || !password) return;
        
        const creds = 'Basic ' + btoa(username + ':' + password);
        
        try {
            const res = await fetch('/auth/check', {
                headers: { Authorization: creds }
            });

            if (res.ok) {
                authHeader.set(creds);
                isAuthenticated.set(true);
                isLoginVisible.set(false);
                error = '';
            } else {
                error = 'Invalid credentials';
            }
        } catch (e) {
            error = 'Login failed';
        }
    }
</script>

{#if $isLoginVisible}
<div id="login-overlay">
    <div id="login-box">
        <h2>Login Required</h2>
        <form on:submit|preventDefault={handleLogin} id="login-form">
            <input
                type="text"
                bind:value={username}
                placeholder="Username"
                required
            />
            <input
                type="password"
                bind:value={password}
                placeholder="Password"
                required
            />
            <button type="submit">Login</button>
        </form>
        {#if error}
            <p id="login-error" style="color: var(--danger)">{error}</p>
        {/if}
    </div>
</div>
{/if}

<style>
    #login-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.8);
        backdrop-filter: blur(8px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 50;
    }
    
    #login-box {
        background: var(--card-bg);
        border: 1px solid var(--card-border);
        border-radius: 16px;
        padding: 2rem;
        max-width: 400px;
        width: 90%;
        text-align: center;
        box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
    }
    
    h2 {
        margin-bottom: 1.5rem;
    }

    form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }
</style>

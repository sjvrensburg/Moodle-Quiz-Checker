<script lang="ts">
	import "../lib/global.css";
	import { theme, agentServerUrl } from "$lib/stores";
	import { api } from "$lib/api";
	import { onMount } from "svelte";

	let { children } = $props();
	let starting = $state(false);

	onMount(() => {
		document.documentElement.setAttribute("data-theme", $theme);
	});

	$effect(() => {
		document.documentElement.setAttribute("data-theme", $theme);
	});

	async function toggleAgentServer() {
		if ($agentServerUrl) {
			agentServerUrl.set(null);
			return;
		}
		starting = true;
		try {
			const url = await api.startAgentServer(4173);
			agentServerUrl.set(url);
		} catch (e) {
			alert(`Failed to start agent server: ${e}`);
		} finally {
			starting = false;
		}
	}
</script>

<div class="shell">
	<aside class="sidebar">
		<div class="brand">
			<span class="brand-mark">MQ</span>
			<span class="brand-name">Moodle Quiz Tester</span>
		</div>

		<nav>
			<a href="/">Quizzes</a>
		</nav>

		<div class="sidebar-footer">
			<div class="agent-status">
				<span class="dot" class:on={!!$agentServerUrl}></span>
				<span class="agent-label">
					{#if $agentServerUrl}
						Agent server on
					{:else}
						Agent server off
					{/if}
				</span>
			</div>
			<button class="btn" onclick={toggleAgentServer} disabled={starting}>
				{$agentServerUrl ? "Stop" : "Start"} agent API
			</button>
			{#if $agentServerUrl}
				<code class="agent-url">{$agentServerUrl}</code>
			{/if}

			<button class="btn theme-toggle" onclick={() => theme.toggle()}>
				{$theme === "dark" ? "☀️ Light mode" : "🌙 Dark mode"}
			</button>
		</div>
	</aside>

	<main class="content">
		{@render children()}
	</main>
</div>

<style>
	.shell {
		display: flex;
		min-height: 100vh;
	}

	.sidebar {
		width: 240px;
		flex-shrink: 0;
		background: var(--mqt-surface);
		border-right: 1px solid var(--mqt-border);
		display: flex;
		flex-direction: column;
		padding: 1.25em 1em;
		gap: 1.5em;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.6em;
	}

	.brand-mark {
		background: var(--mqt-primary);
		color: white;
		font-weight: 700;
		font-size: 0.8em;
		border-radius: 6px;
		padding: 0.4em 0.55em;
	}

	.brand-name {
		font-weight: 600;
		font-size: 0.95em;
	}

	nav {
		display: flex;
		flex-direction: column;
		gap: 0.25em;
	}

	nav a {
		text-decoration: none;
		color: var(--mqt-text);
		padding: 0.5em 0.6em;
		border-radius: 6px;
		font-weight: 500;
		font-size: 0.92em;
	}

	nav a:hover {
		background: var(--mqt-surface-alt);
	}

	.sidebar-footer {
		margin-top: auto;
		display: flex;
		flex-direction: column;
		gap: 0.6em;
	}

	.agent-status {
		display: flex;
		align-items: center;
		gap: 0.5em;
		font-size: 0.85em;
		color: var(--mqt-text-muted);
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--mqt-text-muted);
	}

	.dot.on {
		background: var(--mqt-correct);
	}

	.agent-url {
		font-size: 0.78em;
		color: var(--mqt-text-muted);
		word-break: break-all;
	}

	.content {
		flex: 1;
		min-width: 0;
		padding: 2em 2.5em;
	}
</style>

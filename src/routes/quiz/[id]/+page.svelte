<script lang="ts">
	import { page } from "$app/stores";
	import { goto } from "$app/navigation";
	import { onMount } from "svelte";
	import { api } from "$lib/api";
	import { sanitizeHtml } from "$lib/sanitize";
	import type { Attempt, Quiz } from "$lib/types";

	let quiz = $state<Quiz | null>(null);
	let attempts = $state<Attempt[]>([]);
	let shuffle = $state(true);
	let starting = $state(false);
	let error = $state<string | null>(null);

	const quizId = $page.params.id as string;

	async function load() {
		try {
			quiz = await api.getQuiz(quizId);
			attempts = await api.listAttempts(quizId);
		} catch (e) {
			error = String(e);
		}
	}

	onMount(load);

	async function startAttempt() {
		starting = true;
		try {
			const attempt = await api.startAttempt(quizId, shuffle);
			goto(`/attempt/${attempt.id}`);
		} catch (e) {
			error = String(e);
		} finally {
			starting = false;
		}
	}

	function qtypeLabel(t: string) {
		return t.replace(/_/g, " ");
	}
</script>

<a href="/" class="back">← Quizzes</a>

{#if error}
	<p class="error">{error}</p>
{/if}

{#if quiz}
	<h1>{quiz.name}</h1>
	<p class="muted">{quiz.questions.length} questions · imported {new Date(quiz.imported_at).toLocaleString()}</p>

	<div class="card start-card">
		<label class="shuffle-toggle">
			<input type="checkbox" bind:checked={shuffle} />
			Shuffle question order
		</label>
		<button class="btn btn-primary" onclick={startAttempt} disabled={starting}>
			{starting ? "Starting…" : "Start new attempt"}
		</button>
	</div>

	<h2>Questions</h2>
	<ol class="question-list">
		{#each quiz.questions as q (q.id)}
			<li>
				<div class="q-name">{q.name}</div>
				<div class="q-meta">
					<span class="badge badge-muted">{qtypeLabel(q.qtype)}</span>
					<span class="muted">{q.default_grade} pts</span>
				</div>
				<div class="q-text">{@html sanitizeHtml(q.question_text)}</div>
			</li>
		{/each}
	</ol>

	{#if attempts.length > 0}
		<h2>Past attempts</h2>
		<table class="attempts-table">
			<thead>
				<tr>
					<th>Started</th>
					<th>Status</th>
					<th>Score</th>
					<th></th>
				</tr>
			</thead>
			<tbody>
				{#each attempts as a (a.id)}
					<tr>
						<td>{new Date(a.started_at).toLocaleString()}</td>
						<td>{a.finished_at ? "Finished" : "In progress"}</td>
						<td>
							{#if a.total_score !== null && a.max_score !== null}
								{a.total_score.toFixed(2)} / {a.max_score.toFixed(2)}
							{:else}
								—
							{/if}
						</td>
						<td><a class="btn" href={`/attempt/${a.id}`}>Open</a></td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
{/if}

<style>
	.back {
		display: inline-block;
		margin-bottom: 1em;
		font-size: 0.9em;
		text-decoration: none;
	}

	.muted {
		color: var(--mqt-text-muted);
	}

	.error {
		color: var(--mqt-incorrect);
	}

	.start-card {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin: 1.25em 0 2em;
	}

	.shuffle-toggle {
		display: flex;
		align-items: center;
		gap: 0.5em;
		font-size: 0.92em;
	}

	.question-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75em;
	}

	.question-list li {
		background: var(--mqt-surface);
		border: 1px solid var(--mqt-border);
		border-radius: var(--mqt-radius);
		padding: 1em 1.2em;
	}

	.q-name {
		font-weight: 600;
	}

	.q-meta {
		display: flex;
		gap: 0.6em;
		align-items: center;
		margin: 0.3em 0 0.5em;
		font-size: 0.85em;
	}

	.q-text {
		font-size: 0.92em;
		color: var(--mqt-text-muted);
	}

	.attempts-table {
		width: 100%;
		border-collapse: collapse;
		margin-top: 1em;
	}

	.attempts-table th,
	.attempts-table td {
		text-align: left;
		padding: 0.6em 0.8em;
		border-bottom: 1px solid var(--mqt-border);
		font-size: 0.9em;
	}
</style>

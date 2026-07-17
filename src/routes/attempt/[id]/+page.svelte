<script lang="ts">
	import { page } from "$app/stores";
	import { onDestroy, onMount } from "svelte";
	import { api } from "$lib/api";
	import QuestionRenderer from "$lib/components/QuestionRenderer.svelte";
	import type { Attempt, Quiz, ResponseValue } from "$lib/types";

	const attemptId = $page.params.id as string;

	let attempt = $state<Attempt | null>(null);
	let quiz = $state<Quiz | null>(null);
	let currentIndex = $state(0);
	let error = $state<string | null>(null);
	let finishing = $state(false);
	let elapsedSeconds = $state(0);
	let timerHandle: ReturnType<typeof setInterval> | undefined;

	const finished = $derived(!!attempt?.finished_at);

	async function load() {
		try {
			attempt = await api.getAttempt(attemptId);
			quiz = await api.getQuiz(attempt.quiz_id);
		} catch (e) {
			error = String(e);
		}
	}

	onMount(async () => {
		await load();
		timerHandle = setInterval(() => {
			if (attempt && !attempt.finished_at) {
				const started = new Date(attempt.started_at).getTime();
				elapsedSeconds = Math.max(0, Math.floor((Date.now() - started) / 1000));
			}
		}, 1000);
	});

	onDestroy(() => {
		if (timerHandle) clearInterval(timerHandle);
	});

	function formatElapsed(s: number) {
		const m = Math.floor(s / 60)
			.toString()
			.padStart(2, "0");
		const sec = (s % 60).toString().padStart(2, "0");
		return `${m}:${sec}`;
	}

	const currentQuestionId = $derived(attempt ? attempt.question_order[currentIndex] : null);
	const currentQuestion = $derived(quiz && currentQuestionId ? quiz.questions.find((q) => q.id === currentQuestionId) : null);
	const currentResult = $derived(
		attempt?.results?.find((r) => r.question_id === currentQuestionId) ?? null
	);

	async function onAnswer(value: ResponseValue) {
		if (!attempt || !currentQuestionId || finished) return;
		attempt = await api.submitResponse(attempt.id, currentQuestionId, value);
	}

	async function onFlag(flag: boolean) {
		if (!attempt || !currentQuestionId) return;
		attempt = await api.setFlag(attempt.id, currentQuestionId, flag);
	}

	async function finish() {
		if (!attempt) return;
		if (!confirm("Finish this attempt and see your results?")) return;
		finishing = true;
		try {
			attempt = await api.finishAttempt(attempt.id);
		} catch (e) {
			error = String(e);
		} finally {
			finishing = false;
		}
	}

	function statusFor(qid: string): "answered" | "flagged" | "empty" | "correct" | "incorrect" | "partial" {
		if (attempt?.results) {
			const r = attempt.results.find((r) => r.question_id === qid);
			if (r) {
				if (r.state === "correct") return "correct";
				if (r.state === "incorrect") return "incorrect";
				if (r.state === "partially_correct") return "partial";
			}
		}
		const resp = attempt?.responses[qid];
		if (resp?.flagged) return "flagged";
		if (resp && hasValue(resp.value)) return "answered";
		return "empty";
	}

	function hasValue(v: ResponseValue) {
		if (v === null) return false;
		if (typeof v === "string") return v.trim().length > 0;
		if (Array.isArray(v)) return v.length > 0;
		return Object.keys(v).length > 0;
	}

	async function downloadExport(format: "json" | "md") {
		if (!attempt) return;
		let content: string;
		let mime: string;
		if (format === "json") {
			content = JSON.stringify(await api.exportJson(attempt.id), null, 2);
			mime = "application/json";
		} else {
			content = await api.exportMarkdown(attempt.id);
			mime = "text/markdown";
		}
		const blob = new Blob([content], { type: mime });
		const url = URL.createObjectURL(blob);
		const a = document.createElement("a");
		a.href = url;
		a.download = `attempt-${attempt.id}.${format}`;
		a.click();
		URL.revokeObjectURL(url);
	}
</script>

{#if error}
	<p class="error">{error}</p>
{/if}

{#if quiz && attempt}
	<div class="attempt-layout">
		<aside class="nav-panel">
			<h2 class="quiz-title">{quiz.name}</h2>

			{#if !finished}
				<div class="timer">⏱ {formatElapsed(elapsedSeconds)}</div>
			{:else}
				<div class="score">
					{attempt.total_score?.toFixed(2)} / {attempt.max_score?.toFixed(2)}
				</div>
			{/if}

			<div class="nav-grid">
				{#each attempt.question_order as qid, i (qid)}
					<button
						class="nav-cell status-{statusFor(qid)}"
						class:active={i === currentIndex}
						onclick={() => (currentIndex = i)}
					>
						{i + 1}
					</button>
				{/each}
			</div>

			{#if !finished}
				<button class="btn btn-primary finish-btn" onclick={finish} disabled={finishing}>
					{finishing ? "Grading…" : "Finish attempt"}
				</button>
			{:else}
				<div class="export-buttons">
					<button class="btn" onclick={() => downloadExport("json")}>Export JSON</button>
					<button class="btn" onclick={() => downloadExport("md")}>Export Markdown</button>
					<button class="btn" onclick={() => window.print()}>Print / Save as PDF</button>
				</div>
			{/if}
		</aside>

		<div class="question-panel">
			{#if currentQuestion}
				{#key currentQuestion.id}
					<QuestionRenderer
						question={currentQuestion}
						response={attempt.responses[currentQuestion.id]}
						result={currentResult}
						readonly={finished}
						index={currentIndex}
						{onAnswer}
						{onFlag}
					/>
				{/key}
			{/if}

			<div class="pager">
				<button class="btn" disabled={currentIndex === 0} onclick={() => currentIndex--}>← Previous</button>
				<button
					class="btn"
					disabled={currentIndex === attempt.question_order.length - 1}
					onclick={() => currentIndex++}
				>
					Next →
				</button>
			</div>
		</div>

		{#if finished}
			<div class="print-only">
				<h1>{quiz.name}</h1>
				<p>Score: {attempt.total_score?.toFixed(2)} / {attempt.max_score?.toFixed(2)}</p>
				{#each attempt.question_order as qid, i (qid)}
					{@const q = quiz.questions.find((x) => x.id === qid)}
					{#if q}
						<QuestionRenderer
							question={q}
							response={attempt.responses[qid]}
							result={attempt.results?.find((r) => r.question_id === qid) ?? null}
							readonly={true}
							index={i}
							onAnswer={() => {}}
							onFlag={() => {}}
						/>
					{/if}
				{/each}
			</div>
		{/if}
	</div>
{/if}

<style>
	.error {
		color: var(--mqt-incorrect);
	}

	.attempt-layout {
		display: flex;
		gap: 2em;
		align-items: flex-start;
	}

	.nav-panel {
		width: 220px;
		flex-shrink: 0;
		position: sticky;
		top: 1em;
	}

	.quiz-title {
		font-size: 1.05em;
		margin: 0 0 0.5em;
	}

	.timer,
	.score {
		font-weight: 600;
		margin-bottom: 1em;
	}

	.nav-grid {
		display: grid;
		grid-template-columns: repeat(5, 1fr);
		gap: 0.4em;
		margin-bottom: 1.25em;
	}

	.nav-cell {
		aspect-ratio: 1;
		border: 1px solid var(--mqt-border);
		border-radius: 6px;
		background: var(--mqt-surface);
		color: var(--mqt-text);
		font-size: 0.85em;
	}

	.nav-cell.active {
		outline: 2px solid var(--mqt-primary);
	}

	.status-answered {
		background: var(--mqt-surface-alt);
		border-color: var(--mqt-primary);
	}

	.status-flagged {
		border-color: var(--mqt-flag);
		color: var(--mqt-flag);
	}

	.status-correct {
		background: var(--mqt-correct-bg);
		color: var(--mqt-correct);
		border-color: var(--mqt-correct);
	}

	.status-incorrect {
		background: var(--mqt-incorrect-bg);
		color: var(--mqt-incorrect);
		border-color: var(--mqt-incorrect);
	}

	.status-partial {
		background: var(--mqt-partial-bg);
		color: var(--mqt-partial);
		border-color: var(--mqt-partial);
	}

	.finish-btn {
		width: 100%;
	}

	.export-buttons {
		display: flex;
		flex-direction: column;
		gap: 0.5em;
	}

	.question-panel {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1.25em;
	}

	.pager {
		display: flex;
		justify-content: space-between;
	}

	.print-only {
		display: none;
	}

	@media print {
		.nav-panel,
		.question-panel {
			display: none;
		}

		.attempt-layout {
			display: block;
		}

		.print-only {
			display: flex;
			flex-direction: column;
			gap: 1em;
		}
	}
</style>

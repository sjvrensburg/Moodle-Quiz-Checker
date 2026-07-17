<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import { api } from "$lib/api";
	import type { Quiz } from "$lib/types";
	import { open } from "@tauri-apps/plugin-dialog";
	import { readTextFile } from "@tauri-apps/plugin-fs";

	let quizzes = $state<Quiz[]>([]);
	let loading = $state(true);
	let dragOver = $state(false);
	let importing = $state(false);
	let error = $state<string | null>(null);

	async function refresh() {
		loading = true;
		try {
			quizzes = await api.listQuizzes();
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	onMount(refresh);

	async function importFile(file: File) {
		importing = true;
		error = null;
		try {
			const xml = await file.text();
			const name = file.name.replace(/\.xml$/i, "");
			await api.importQuizXml(xml, name, file.name);
			await refresh();
		} catch (e) {
			error = String(e);
		} finally {
			importing = false;
		}
	}

	async function handleDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		const file = e.dataTransfer?.files?.[0];
		if (file) await importFile(file);
	}

	async function browseFile() {
		try {
			const selected = await open({
				multiple: false,
				filters: [{ name: "Moodle XML", extensions: ["xml"] }]
			});
			if (!selected || Array.isArray(selected)) return;
			const xml = await readTextFile(selected);
			const name = selected.split(/[\\/]/).pop()?.replace(/\.xml$/i, "") ?? "Untitled quiz";
			importing = true;
			await api.importQuizXml(xml, name, selected);
			await refresh();
		} catch (e) {
			error = String(e);
		} finally {
			importing = false;
		}
	}

	async function removeQuiz(id: string) {
		if (!confirm("Delete this quiz and all its attempts?")) return;
		await api.deleteQuiz(id);
		await refresh();
	}
</script>

<h1>Quizzes</h1>

<div
	class="dropzone"
	role="region"
	aria-label="Quiz import drop zone"
	class:over={dragOver}
	ondragover={(e) => {
		e.preventDefault();
		dragOver = true;
	}}
	ondragleave={() => (dragOver = false)}
	ondrop={handleDrop}
>
	<p>Drag &amp; drop a Moodle XML export here</p>
	<p class="or">or</p>
	<button class="btn btn-primary" onclick={browseFile} disabled={importing}>
		{importing ? "Importing…" : "Browse for file"}
	</button>
</div>

{#if error}
	<p class="error">{error}</p>
{/if}

{#if loading}
	<p>Loading…</p>
{:else if quizzes.length === 0}
	<p class="muted">No quizzes imported yet.</p>
{:else}
	<div class="grid">
		{#each quizzes as quiz (quiz.id)}
			<div class="card quiz-card">
				<h3>
					<a href={`/quiz/${quiz.id}`}>{quiz.name}</a>
				</h3>
				<p class="muted">{quiz.questions.length} questions</p>
				<p class="muted small">Imported {new Date(quiz.imported_at).toLocaleString()}</p>
				<div class="row">
					<a class="btn btn-primary" href={`/quiz/${quiz.id}`}>Open</a>
					<button class="btn" onclick={() => removeQuiz(quiz.id)}>Delete</button>
				</div>
			</div>
		{/each}
	</div>
{/if}

<style>
	.dropzone {
		border: 2px dashed var(--mqt-border);
		border-radius: var(--mqt-radius);
		padding: 2.5em;
		text-align: center;
		margin: 1.25em 0 2em;
		color: var(--mqt-text-muted);
	}

	.dropzone.over {
		border-color: var(--mqt-primary);
		background: var(--mqt-surface-alt);
	}

	.or {
		font-size: 0.85em;
		margin: 0.5em 0;
	}

	.error {
		color: var(--mqt-incorrect);
	}

	.muted {
		color: var(--mqt-text-muted);
	}

	.small {
		font-size: 0.82em;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
		gap: 1em;
	}

	.quiz-card h3 {
		margin: 0 0 0.3em;
	}

	.row {
		display: flex;
		gap: 0.5em;
		margin-top: 0.8em;
	}
</style>

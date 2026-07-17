<script lang="ts">
	import { sanitizeHtml } from "$lib/sanitize";
	import type { Question, ResponseValue } from "$lib/types";

	let {
		question,
		value,
		readonly,
		onChange
	}: { question: Question; value: ResponseValue; readonly: boolean; onChange: (v: ResponseValue) => void } =
		$props();

	const mapping = $derived(
		value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, string>) : {}
	);

	// Shuffle once per mount (stable across re-renders / keystrokes).
	const shuffledOptions = (() => {
		const opts = question.match_pairs.map((p) => p.answer_text);
		if (!question.shuffle_answers) return opts;
		const arr = [...opts];
		for (let i = arr.length - 1; i > 0; i--) {
			const j = Math.floor(Math.random() * (i + 1));
			[arr[i], arr[j]] = [arr[j], arr[i]];
		}
		return arr;
	})();

	function setMatch(pairId: string, answerText: string) {
		if (readonly) return;
		onChange({ ...mapping, [pairId]: answerText });
	}
</script>

<div class="matching">
	{#each question.match_pairs as pair (pair.id)}
		<div class="match-row">
			<div class="stem">{@html sanitizeHtml(pair.question_text)}</div>
			<select
				disabled={readonly}
				value={mapping[pair.id] ?? ""}
				onchange={(e) => setMatch(pair.id, (e.target as HTMLSelectElement).value)}
			>
				<option value="">Choose…</option>
				{#each shuffledOptions as opt}
					<option value={opt}>{opt}</option>
				{/each}
			</select>
		</div>
	{/each}
</div>

<style>
	.matching {
		display: flex;
		flex-direction: column;
		gap: 0.5em;
	}

	.match-row {
		display: flex;
		align-items: center;
		gap: 1em;
	}

	.stem {
		min-width: 180px;
	}

	.stem :global(p) {
		margin: 0;
	}

	select {
		min-width: 200px;
	}
</style>

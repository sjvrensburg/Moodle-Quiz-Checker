<script lang="ts">
	import { sanitizeHtml } from "$lib/sanitize";
	import type { GradeState } from "$lib/types";

	let { state, feedback }: { state: GradeState; feedback: string | null } = $props();

	const labels: Record<GradeState, string> = {
		correct: "Correct",
		partially_correct: "Partially correct",
		incorrect: "Incorrect",
		ungraded: "Not graded"
	};
</script>

<div class="feedback feedback-{state}">
	<span class="badge badge-{state === 'correct' ? 'correct' : state === 'incorrect' ? 'incorrect' : state === 'partially_correct' ? 'partial' : 'muted'}">
		{labels[state]}
	</span>
	{#if feedback}
		<div class="feedback-text">{@html sanitizeHtml(feedback)}</div>
	{/if}
</div>

<style>
	.feedback {
		margin-top: 0.75em;
		padding: 0.75em 1em;
		border-radius: var(--mqt-radius);
		background: var(--mqt-surface-alt);
	}

	.feedback-correct {
		background: var(--mqt-correct-bg);
	}

	.feedback-incorrect {
		background: var(--mqt-incorrect-bg);
	}

	.feedback-partially_correct {
		background: var(--mqt-partial-bg);
	}

	.feedback-text {
		margin-top: 0.4em;
		font-size: 0.92em;
	}
</style>

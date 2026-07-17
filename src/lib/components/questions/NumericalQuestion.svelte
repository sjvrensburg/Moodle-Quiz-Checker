<script lang="ts">
	import type { Question, ResponseValue } from "$lib/types";

	let {
		question,
		value,
		readonly,
		onChange
	}: { question: Question; value: ResponseValue; readonly: boolean; onChange: (v: ResponseValue) => void } =
		$props();

	const text = $derived(typeof value === "string" ? value : "");
	const unitLabel = $derived(question.numerical_units.map((u) => u[0]).filter(Boolean).join(" / "));
</script>

<div class="numerical-row">
	<input
		type="text"
		inputmode="decimal"
		class="answer-input"
		placeholder="Enter a number…"
		value={text}
		disabled={readonly}
		oninput={(e) => onChange((e.target as HTMLInputElement).value)}
	/>
	{#if unitLabel}
		<span class="unit">{unitLabel}</span>
	{/if}
</div>

<style>
	.numerical-row {
		display: flex;
		align-items: center;
		gap: 0.6em;
	}

	.answer-input {
		max-width: 220px;
	}

	.unit {
		color: var(--mqt-text-muted);
		font-size: 0.9em;
	}
</style>

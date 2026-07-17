<script lang="ts">
	import { sanitizeHtml } from "$lib/sanitize";
	import type { Answer, Question, ResponseValue } from "$lib/types";

	let {
		question,
		value,
		readonly,
		onChange,
		trueFalse = false
	}: {
		question: Question;
		value: ResponseValue;
		readonly: boolean;
		onChange: (v: ResponseValue) => void;
		trueFalse?: boolean;
	} = $props();

	const selected = $derived(Array.isArray(value) ? value : typeof value === "string" ? [value] : []);

	function isChecked(a: Answer) {
		return selected.includes(a.id);
	}

	function choose(a: Answer) {
		if (readonly) return;
		if (question.single) {
			onChange([a.id]);
		} else {
			const set = new Set(selected);
			if (set.has(a.id)) set.delete(a.id);
			else set.add(a.id);
			onChange(Array.from(set));
		}
	}

	function label(a: Answer) {
		if (trueFalse) return a.text.toLowerCase() === "true" ? "True" : "False";
		return a.text;
	}
</script>

<div class="options" role="group">
	{#each question.answers as a (a.id)}
		<label class="option" class:checked={isChecked(a)} class:readonly>
			<input
				type={question.single ? "radio" : "checkbox"}
				name={`q-${question.id}`}
				checked={isChecked(a)}
				disabled={readonly}
				onclick={() => choose(a)}
			/>
			{#if trueFalse}
				<span class="option-text">{label(a)}</span>
			{:else}
				<span class="option-text">{@html sanitizeHtml(a.text)}</span>
			{/if}
		</label>
	{/each}
</div>

<style>
	.options {
		display: flex;
		flex-direction: column;
		gap: 0.4em;
	}

	.option {
		display: flex;
		align-items: flex-start;
		gap: 0.6em;
		padding: 0.55em 0.75em;
		border: 1px solid var(--mqt-border);
		border-radius: 6px;
		cursor: pointer;
	}

	.option.readonly {
		cursor: default;
	}

	.option.checked {
		border-color: var(--mqt-primary);
		background: var(--mqt-surface-alt);
	}

	.option input {
		margin-top: 0.2em;
	}

	.option-text :global(p) {
		margin: 0;
	}
</style>

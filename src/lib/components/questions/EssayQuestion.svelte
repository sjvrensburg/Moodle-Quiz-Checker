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
	const rows = $derived(question.essay_lines ?? 10);
</script>

<textarea
	{rows}
	placeholder="Write your response…"
	value={text}
	disabled={readonly}
	oninput={(e) => onChange((e.target as HTMLTextAreaElement).value)}
></textarea>

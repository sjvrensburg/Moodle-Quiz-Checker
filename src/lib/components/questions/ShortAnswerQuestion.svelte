<script lang="ts">
	import type { ResponseValue } from "$lib/types";

	let {
		value,
		readonly,
		onChange,
		placeholder = "Type your answer…"
	}: { value: ResponseValue; readonly: boolean; onChange: (v: ResponseValue) => void; placeholder?: string } =
		$props();

	const text = $derived(typeof value === "string" ? value : "");
</script>

<input
	type="text"
	class="answer-input"
	{placeholder}
	value={text}
	disabled={readonly}
	oninput={(e) => onChange((e.target as HTMLInputElement).value)}
/>

<style>
	.answer-input {
		width: 100%;
		max-width: 420px;
	}
</style>

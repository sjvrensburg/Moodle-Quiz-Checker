<script lang="ts">
	import { sanitizeHtml } from "$lib/sanitize";
	import type { ClozeItem, Question, ResponseValue } from "$lib/types";

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

	const CLOZE_RE = /\{(\d*):([A-Za-z_]+):([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}/g;

	type Segment = { html: string } | { item: ClozeItem };

	const segments: Segment[] = (() => {
		const out: Segment[] = [];
		let lastIndex = 0;
		let itemIdx = 0;
		let m: RegExpExecArray | null;
		CLOZE_RE.lastIndex = 0;
		while ((m = CLOZE_RE.exec(question.question_text))) {
			if (m.index > lastIndex) {
				out.push({ html: question.question_text.slice(lastIndex, m.index) });
			}
			const item = question.cloze_items[itemIdx];
			itemIdx++;
			if (item) out.push({ item });
			lastIndex = m.index + m[0].length;
		}
		if (lastIndex < question.question_text.length) {
			out.push({ html: question.question_text.slice(lastIndex) });
		}
		return out;
	})();

	function setValue(itemIndex: number, v: string) {
		if (readonly) return;
		onChange({ ...mapping, [String(itemIndex)]: v });
	}
</script>

<div class="cloze-text">
	{#each segments as seg}
		{#if "html" in seg}
			{@html sanitizeHtml(seg.html)}
		{:else if seg.item.kind === "MULTICHOICE_INLINE" || seg.item.kind === "MULTICHOICE_DROPDOWN"}
			<select
				class="cloze-widget"
				disabled={readonly}
				value={mapping[String(seg.item.index)] ?? ""}
				onchange={(e) => setValue(seg.item.index, (e.target as HTMLSelectElement).value)}
			>
				<option value="">…</option>
				{#each seg.item.options as opt}
					<option value={opt.id}>{opt.text}</option>
				{/each}
			</select>
		{:else}
			<input
				type="text"
				class="cloze-widget cloze-input"
				disabled={readonly}
				value={mapping[String(seg.item.index)] ?? ""}
				oninput={(e) => setValue(seg.item.index, (e.target as HTMLInputElement).value)}
			/>
		{/if}
	{/each}
</div>

<style>
	.cloze-text {
		line-height: 2.2;
	}

	.cloze-text :global(p) {
		display: inline;
	}

	.cloze-widget {
		margin: 0 0.2em;
	}

	.cloze-input {
		width: 8em;
	}
</style>

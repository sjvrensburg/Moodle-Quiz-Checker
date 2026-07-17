<script lang="ts">
	import { sanitizeHtml } from "$lib/sanitize";
	import { attachmentSlug, downloadAttachment } from "$lib/attachments";
	import type { Question, QuestionResult, Response, ResponseValue } from "$lib/types";
	import FeedbackBanner from "./FeedbackBanner.svelte";
	import MultiChoiceQuestion from "./questions/MultiChoiceQuestion.svelte";
	import ShortAnswerQuestion from "./questions/ShortAnswerQuestion.svelte";
	import NumericalQuestion from "./questions/NumericalQuestion.svelte";
	import MatchingQuestion from "./questions/MatchingQuestion.svelte";
	import ClozeQuestion from "./questions/ClozeQuestion.svelte";
	import EssayQuestion from "./questions/EssayQuestion.svelte";

	let {
		question,
		response,
		result = null,
		readonly = false,
		index,
		onAnswer,
		onFlag
	}: {
		question: Question;
		response: Response | undefined;
		result?: QuestionResult | null;
		readonly?: boolean;
		index: number;
		onAnswer: (v: ResponseValue) => void;
		onFlag: (flagged: boolean) => void;
	} = $props();

	const value = $derived<ResponseValue>(response?.value ?? null);
	const flagged = $derived(response?.flagged ?? false);
</script>

<div class="question-block">
	<div class="q-header">
		<h3>Question {index + 1}. {question.name}</h3>
		<button
			class="flag-btn"
			class:flagged
			title={flagged ? "Unflag question" : "Flag question for review"}
			onclick={() => onFlag(!flagged)}
		>
			{flagged ? "🚩" : "⚑"}
		</button>
	</div>

	<div class="q-text">{@html sanitizeHtml(question.question_text)}</div>

	{#if question.files.length > 0}
		<div class="attachments">
			<span class="attachments-label">Attachments:</span>
			{#each question.files as file (file.name)}
				<button
					id={`attachment-${attachmentSlug(file.name)}`}
					class="btn attachment-btn"
					onclick={() => downloadAttachment(file)}
				>
					📎 {file.name}
				</button>
			{/each}
		</div>
	{/if}

	<div class="q-body">
		{#if question.qtype === "multi_choice"}
			<MultiChoiceQuestion {question} {value} {readonly} onChange={onAnswer} />
		{:else if question.qtype === "true_false"}
			<MultiChoiceQuestion {question} {value} {readonly} onChange={onAnswer} trueFalse />
		{:else if question.qtype === "short_answer"}
			<ShortAnswerQuestion {value} {readonly} onChange={onAnswer} />
		{:else if question.qtype === "numerical"}
			<NumericalQuestion {question} {value} {readonly} onChange={onAnswer} />
		{:else if question.qtype === "matching"}
			<MatchingQuestion {question} {value} {readonly} onChange={onAnswer} />
		{:else if question.qtype === "cloze"}
			<ClozeQuestion {question} {value} {readonly} onChange={onAnswer} />
		{:else if question.qtype === "essay"}
			<EssayQuestion {question} {value} {readonly} onChange={onAnswer} />
		{:else if question.qtype === "description"}
			<p class="muted">(No response required.)</p>
		{:else}
			<p class="muted">This question type is not supported for interactive attempts.</p>
		{/if}
	</div>

	{#if result && readonly}
		<FeedbackBanner state={result.state} feedback={result.feedback} />
		{#if question.general_feedback}
			<div class="general-feedback">{@html sanitizeHtml(question.general_feedback)}</div>
		{/if}
	{/if}
</div>

<style>
	.question-block {
		background: var(--mqt-surface);
		border: 1px solid var(--mqt-border);
		border-radius: var(--mqt-radius);
		padding: 1.5em 1.75em;
		box-shadow: var(--mqt-shadow);
	}

	.q-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1em;
	}

	.q-header h3 {
		margin: 0;
	}

	.flag-btn {
		background: none;
		border: none;
		font-size: 1.2em;
		color: var(--mqt-text-muted);
		padding: 0.1em 0.3em;
	}

	.flag-btn.flagged {
		color: var(--mqt-flag);
	}

	.q-text {
		margin: 0.75em 0 1.1em;
	}

	.attachments {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5em;
		margin-bottom: 1.1em;
	}

	.attachments-label {
		font-size: 0.85em;
		color: var(--mqt-text-muted);
	}

	.attachment-btn {
		font-size: 0.85em;
		padding: 0.35em 0.7em;
	}

	.q-text :global(p:first-child) {
		margin-top: 0;
	}

	.muted {
		color: var(--mqt-text-muted);
	}

	.general-feedback {
		margin-top: 0.75em;
		font-size: 0.9em;
		color: var(--mqt-text-muted);
	}
</style>

export type TextFormat = "html" | "plain" | "markdown" | "moodle";

export interface Answer {
	id: string;
	text: string;
	format: TextFormat;
	fraction: number;
	feedback: string | null;
}

export interface MatchPair {
	id: string;
	question_text: string;
	answer_text: string;
}

export type ClozeKind =
	| "MULTICHOICE_INLINE"
	| "MULTICHOICE_DROPDOWN"
	| "SHORT_ANSWER"
	| "SHORT_ANSWER_CASE_SENSITIVE"
	| "NUMERICAL";

export interface ClozeItem {
	id: string;
	index: number;
	kind: ClozeKind;
	options: Answer[];
}

export type QuestionType =
	| "multi_choice"
	| "true_false"
	| "short_answer"
	| "numerical"
	| "matching"
	| "cloze"
	| "essay"
	| "description"
	| "unsupported";

export interface QuestionFile {
	name: string;
	data_base64: string;
}

export interface NumericalTolerance {
	value: number;
	tolerance: number;
	fraction: number;
	feedback: string | null;
}

export interface Question {
	id: string;
	qtype: QuestionType;
	name: string;
	question_text: string;
	question_text_format: TextFormat;
	general_feedback: string | null;
	default_grade: number;
	penalty: number;
	category: string | null;
	shuffle_answers: boolean;
	single: boolean;
	answers: Answer[];
	case_sensitive: boolean;
	numerical_answers: NumericalTolerance[];
	numerical_units: [string, number][];
	match_pairs: MatchPair[];
	cloze_items: ClozeItem[];
	correct_feedback: string | null;
	partially_correct_feedback: string | null;
	incorrect_feedback: string | null;
	essay_response_format: string | null;
	essay_lines: number | null;
	files: QuestionFile[];
}

export interface Quiz {
	id: string;
	name: string;
	source_file: string | null;
	questions: Question[];
	imported_at: string;
}

export type ResponseValue = string | string[] | Record<string, string> | null;

export interface Response {
	value: ResponseValue;
	flagged: boolean;
}

export type GradeState = "correct" | "partially_correct" | "incorrect" | "ungraded";

export interface QuestionResult {
	question_id: string;
	fraction: number;
	raw_grade: number;
	max_grade: number;
	feedback: string | null;
	state: GradeState;
}

export interface Attempt {
	id: string;
	quiz_id: string;
	started_at: string;
	finished_at: string | null;
	question_order: string[];
	responses: Record<string, Response>;
	results: QuestionResult[] | null;
	total_score: number | null;
	max_score: number | null;
}

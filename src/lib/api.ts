import { invoke } from "@tauri-apps/api/core";
import type { Attempt, AutotestReport, LintReport, Quiz, ResponseValue } from "./types";

export const api = {
	importQuizXml(xml: string, name: string, sourceFile?: string): Promise<Quiz> {
		return invoke("import_quiz_xml", { xml, name, sourceFile: sourceFile ?? null });
	},
	listQuizzes(): Promise<Quiz[]> {
		return invoke("list_quizzes");
	},
	getQuiz(quizId: string): Promise<Quiz> {
		return invoke("get_quiz", { quizId });
	},
	deleteQuiz(quizId: string): Promise<void> {
		return invoke("delete_quiz", { quizId });
	},
	startAttempt(quizId: string, shuffle: boolean): Promise<Attempt> {
		return invoke("start_attempt", { quizId, shuffle });
	},
	getAttempt(attemptId: string): Promise<Attempt> {
		return invoke("get_attempt", { attemptId });
	},
	submitResponse(attemptId: string, questionId: string, value: ResponseValue): Promise<Attempt> {
		return invoke("submit_response", { attemptId, questionId, value });
	},
	setFlag(attemptId: string, questionId: string, flagged: boolean): Promise<Attempt> {
		return invoke("set_flag", { attemptId, questionId, flagged });
	},
	finishAttempt(attemptId: string): Promise<Attempt> {
		return invoke("finish_attempt", { attemptId });
	},
	listAttempts(quizId: string): Promise<Attempt[]> {
		return invoke("list_attempts", { quizId });
	},
	exportJson(attemptId: string): Promise<unknown> {
		return invoke("export_json", { attemptId });
	},
	exportMarkdown(attemptId: string): Promise<string> {
		return invoke("export_markdown", { attemptId });
	},
	lintXml(xml: string): Promise<LintReport> {
		return invoke("lint_xml", { xml });
	},
	autotestQuiz(quizId: string): Promise<AutotestReport> {
		return invoke("autotest_quiz", { quizId });
	},
	exportQuizMarkdown(quizId: string): Promise<string> {
		return invoke("export_quiz_markdown", { quizId });
	},
	renderQuestionHtml(quizId: string, questionId: string): Promise<string> {
		return invoke("render_question_html", { quizId, questionId });
	},
	startAgentServer(port: number): Promise<string> {
		return invoke("start_agent_server", { port });
	}
};

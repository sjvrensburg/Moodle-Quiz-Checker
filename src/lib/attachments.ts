import type { QuestionFile } from "./types";

/** Must match `attachment_slug` in src-tauri/src/parser.rs. */
export function attachmentSlug(name: string): string {
	return name.replace(/[^a-zA-Z0-9._-]/g, "-");
}

const MIME_BY_EXT: Record<string, string> = {
	csv: "text/csv",
	txt: "text/plain",
	r: "text/x-r-source",
	json: "application/json",
	png: "image/png",
	jpg: "image/jpeg",
	jpeg: "image/jpeg",
	pdf: "application/pdf"
};

function mimeFor(name: string): string {
	const ext = name.split(".").pop()?.toLowerCase() ?? "";
	return MIME_BY_EXT[ext] ?? "application/octet-stream";
}

export function downloadAttachment(file: QuestionFile) {
	const bytes = atob(file.data_base64);
	const buf = new Uint8Array(bytes.length);
	for (let i = 0; i < bytes.length; i++) buf[i] = bytes.charCodeAt(i);
	const blob = new Blob([buf], { type: mimeFor(file.name) });
	const url = URL.createObjectURL(blob);
	const a = document.createElement("a");
	a.href = url;
	a.download = file.name;
	a.click();
	URL.revokeObjectURL(url);
}

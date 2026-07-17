import DOMPurify from "dompurify";

export function sanitizeHtml(html: string | null | undefined): string {
	if (!html) return "";
	return DOMPurify.sanitize(html, {
		ALLOWED_TAGS: [
			"p", "br", "b", "i", "em", "strong", "u", "s", "sub", "sup", "span", "div",
			"ul", "ol", "li", "a", "img", "table", "thead", "tbody", "tr", "td", "th",
			"blockquote", "code", "pre", "h1", "h2", "h3", "h4", "h5", "h6", "hr"
		],
		ALLOWED_ATTR: ["href", "src", "alt", "title", "class", "style", "target", "colspan", "rowspan"]
	});
}

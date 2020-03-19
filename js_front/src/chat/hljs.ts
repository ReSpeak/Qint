import hljs from "highlight.js";

export default function hljs_highlight(code: string, lang?: string) {
	const elem = document.createElement("code");
	elem.classList.add("hljs");

	let res: hljs.IHighlightResultBase;
	try {
		if (lang && /^\w{1,30}$/.test(lang)) {
			res = hljs.highlight(lang, code);
		} else {
			res = hljs.highlightAuto(code);
		}
		elem.setAttribute("rel", res.language);
		elem.innerHTML = res.value;
	} catch {
		elem.innerText = code;
	}
	return elem;
}

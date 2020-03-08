// @ts-check
import hljs from 'highlight.js';

/**
 * @param {string} code
 * @param {string} lang
 */
export default function hljs_highlight(code, lang) {
	const elem = document.createElement("code");
	elem.classList.add("hljs");

	/** @type {hljs.IHighlightResultBase} */
	let res;
	try {
		if (/^\w{1,30}$/.test(lang)) {
			res = hljs.highlight(lang, code);
		} else {
			res = hljs.highlightAuto(code);
		}
		elem.classList.add("language-" + res.language);
		elem.innerHTML = res.value;
	} catch {
		elem.innerText = code;
	}
	return elem;
}

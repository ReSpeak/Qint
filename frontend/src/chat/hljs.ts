import hljs from "highlight.js";

export function hljsHighlight(elem: HTMLElement) {
	const lang = elem.getAttribute('data-lang');
	const hl_lang = lang ? hljs.getLanguage(lang) : undefined;

	const code = elem.textContent ?? "";

	let res: HighlightResult;
	try {
		if (hl_lang?.name !== undefined) {
			res = hljs.highlight(hl_lang.name, code);
		} else {
			res = hljs.highlightAuto(code);
		}
		if (res.language !== undefined) {
			// Add the language name to the ovarlay if a language was found
			if (elem.parentElement) {
				elem.parentElement.dataset["codelang"] = res.language;
			}
			// Add the class for language specific highlighting
			elem.classList.add("lang-" + res.language);
		}
		elem.innerHTML = res.value;
		// Set hljs class for correct highlighting
		elem.classList.add("hljs");
	} catch {
		elem.innerText = code;
	}
}

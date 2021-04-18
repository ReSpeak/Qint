import hljs from 'highlight.js/lib/core.js';

import lang_c_like from 'highlight.js/lib/languages/c-like';
import lang_cpp from 'highlight.js/lib/languages/cpp';
import lang_xml from 'highlight.js/lib/languages/xml';
import lang_bash from 'highlight.js/lib/languages/bash';
import lang_c from 'highlight.js/lib/languages/c';
import lang_csharp from 'highlight.js/lib/languages/csharp';
import lang_css from 'highlight.js/lib/languages/css';
import lang_markdown from 'highlight.js/lib/languages/markdown';
import lang_diff from 'highlight.js/lib/languages/diff';
import lang_haxe from 'highlight.js/lib/languages/haxe';
import lang_ini from 'highlight.js/lib/languages/ini';
import lang_java from 'highlight.js/lib/languages/java';
import lang_javascript from 'highlight.js/lib/languages/javascript';
import lang_json from 'highlight.js/lib/languages/json';
import lang_latex from 'highlight.js/lib/languages/latex';
import lang_less from 'highlight.js/lib/languages/less';
import lang_lua from 'highlight.js/lib/languages/lua';
import lang_nix from 'highlight.js/lib/languages/nix';
import lang_php from 'highlight.js/lib/languages/php';
import lang_powershell from 'highlight.js/lib/languages/powershell';
import lang_python from 'highlight.js/lib/languages/python';
import lang_rust from 'highlight.js/lib/languages/rust';
import lang_scss from 'highlight.js/lib/languages/scss';
import lang_sql from 'highlight.js/lib/languages/sql';
import lang_yaml from 'highlight.js/lib/languages/yaml';
import lang_typescript from 'highlight.js/lib/languages/typescript';

hljs.registerLanguage("c-like", lang_c_like);
hljs.registerLanguage("cpp", lang_cpp);
hljs.registerLanguage("xml", lang_xml);
hljs.registerLanguage("bash", lang_bash);
hljs.registerLanguage("c", lang_c);
hljs.registerLanguage("csharp", lang_csharp);
hljs.registerLanguage("css", lang_css);
hljs.registerLanguage("markdown", lang_markdown);
hljs.registerLanguage("diff", lang_diff);
hljs.registerLanguage("haxe", lang_haxe);
hljs.registerLanguage("ini", lang_ini);
hljs.registerLanguage("java", lang_java);
hljs.registerLanguage("javascript", lang_javascript);
hljs.registerLanguage("json", lang_json);
hljs.registerLanguage("latex", lang_latex);
hljs.registerLanguage("less", lang_less);
hljs.registerLanguage("lua", lang_lua);
hljs.registerLanguage("nix", lang_nix);
hljs.registerLanguage("php", lang_php);
hljs.registerLanguage("powershell", lang_powershell);
hljs.registerLanguage("python", lang_python);
hljs.registerLanguage("rust", lang_rust);
hljs.registerLanguage("scss", lang_scss);
hljs.registerLanguage("sql", lang_sql);
hljs.registerLanguage("yaml", lang_yaml);
hljs.registerLanguage("typescript", lang_typescript);

// Needed for typescript
import type hljs_type from "highlight.js";
const _unused: typeof hljs_type = undefined!;

export function hljsHighlight(elem: HTMLElement): void {
	const lang = elem.getAttribute('data-lang');
	const hl_lang = lang ? hljs.getLanguage(lang) : undefined;

	const code = elem.textContent ?? "";
	let res: HighlightResult;
	try {
		if (hl_lang !== undefined) {
			res = hljs.highlight(code, { language: lang! });
		} else {
			res = hljs.highlightAuto(code);
		}
		if (res.language !== undefined) {
			// Add the language name to the overlay if a language was found
			if (elem.parentElement && elem.parentElement.tagName === "PRE") {
				elem.parentElement.dataset["codelang"] = res.language;
			}
			// Add the class for language specific highlighting
			elem.classList.add("lang-" + res.language);
		}
		elem.innerHTML = res.value;
		// Set hljs class for correct highlighting
		elem.classList.add("hljs");
	} catch {
		elem.textContent = code;
	}
}

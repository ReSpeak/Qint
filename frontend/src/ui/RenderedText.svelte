<script lang="typescript">
	import katex from "katex";
	import { hljsHighlight } from "./hljs";
	import { onMount } from "svelte";
	import { parseTsScheme, schemeToLink } from "./renderedTextDecl";
	import type { LinksMap } from "./renderedTextDecl";
	import type { Connection } from "../connection";

	export let connection: Connection;
	export let text: string;
	export let links: LinksMap = new Map();

	let rendered!: HTMLElement;
	$: renderedObj = render(text);

	function render(html: string) {
		const obj = document.createElement("div");
		obj.innerHTML = html;

		// Apply highlight.js
		for (let elem of obj.getElementsByTagName("code")) {
			hljsHighlight(elem);
		}

		// Apply KaTeX
		for (let elem of (obj.getElementsByClassName("latex") as any) as HTMLElement[]) {
			const code = elem.getAttribute("data-latex");
			const mode = elem.getAttribute("data-displaymode");
			try {
				if (code) {
					katex.render(code, elem, {
						displayMode: mode === "true",
						throwOnError: false,
					});
				}
			} catch {
				console.error("Failed to render latex");
				elem.textContent = code ?? "";
			}
		}

		links.clear();

		// Process links and images
		for (const a of obj.querySelectorAll("a")) {
			const href = a.href;
			if (!href || links.has(href)) continue;
			links.set(href, {
				link: href,
				title: a.textContent ?? "",
			});
		}

		// process ts3file links
		for (const img of obj.querySelectorAll("img")) {
			const src = img.src;
			if (!src) continue;
			const scheme = parseTsScheme(src);
			if (scheme !== null) {
				const proxyFileSrc = schemeToLink(connection, scheme);
				if (proxyFileSrc === null) {
					img.parentElement?.removeChild(img);
					continue;
				} else {
					img.src = proxyFileSrc;
				}
			}
		}

		if (links.size > 0) {
			links = links;
		}

		if (rendered) {
			rendered.innerHTML = "";
			rendered.appendChild(obj);
		}
		return obj;
	}

	onMount(() => {
		rendered.innerHTML = "";
		rendered.appendChild(renderedObj);
	});
</script>

<div class="textRendered content" bind:this={rendered} />

<style lang="scss">
	.textRendered {
		white-space: pre-wrap;
		word-wrap: break-word;
		margin-bottom: 0 !important;

		:global(.para:not(:last-child)) {
			margin-bottom: 1em;
		}

		:global(code.hljs) {
			display: inline;
			padding: 0.1em;
		}

		:global(pre code.hljs) {
			display: block;
			padding: 0.5em;
			position: relative;
			overflow-x: scroll;
		}

		:global([data-codelang]::before) {
			font-size: 0.85em;
			content: attr(data-codelang);
			position: absolute;
			z-index: 2;
			bottom: 0;
			right: 3px;
			color: $orange;
			font-weight: bold;
			font-family: Sans-Serif;
			text-transform: uppercase;
			pointer-events: none;
		}
	}
</style>

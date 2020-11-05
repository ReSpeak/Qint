<script lang="typescript">
	import katex from "katex";
	import { hljsHighlight } from "./hljs";
	import { onMount } from "svelte";

	export let text: string;
	export let links: [string, string][] = [];

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
				elem.innerText = code ?? "";
			}
		}

		// Process links and images
		links = [...obj.querySelectorAll("a")]
			.filter((a) => !!a.href)
			.map((a) => [a.href, a.innerText]);

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

<div class="textRendered" bind:this={rendered} />

<style lang="scss">
	.textRendered {
		white-space: pre-wrap;
		word-wrap: break-word;
		margin-bottom: 0;

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

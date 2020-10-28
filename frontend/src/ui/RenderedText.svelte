<script lang="typescript">
	import katex from "katex";
	import { hljsHighlight } from "./hljs";
	import { onMount } from "svelte";
	import Icon from "../ui/Icon.svelte";

	export let text: string;
	export let raw: string | undefined = undefined;
	export let links: [string, string][] = [];

	let rendered!: HTMLElement;
	$: renderedObj = render(text);
	let viewRaw = false;

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

<div
	class="textBody"
	class:viewRaw>
	<div class="textRendered" bind:this={rendered} />
	{#if raw !== undefined}
		<div class="textRaw">
			<pre>{raw}</pre>
		</div>
		<div class="tool-buttons">
			<div class="tool-buttons-wrap buttons has-addons">
				<button
					class="button is-small is-rounded"
					on:click={() => (viewRaw = !viewRaw)}
					title="It’s raw!">
					<Icon raw="🥩" />
				</button>
			</div>
		</div>
	{/if}
</div>

<style lang="scss">
	@import "../global_mixin";
	$row-pad: 0.25em;

	.textBody {
		flex: 1;

		// for tool buttons
		position: relative;

		// Overwrite bulma default
		:global(pre) {
			position: relative;
			padding: 0;
			margin: 1em 1em 1em 0;
			border-radius: 7px;
			background: none;

			tab-size: 4;
			-moz-tab-size: 4;
			// TODO Prevent scrollbar
		}

		&:hover {
			.tool-buttons {
				visibility: visible;
			}
		}
	}

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

	.textRaw > pre {
		overflow-y: hidden;
		background: none;
		margin: 0;
	}

	.tool-buttons {
		visibility: hidden;
		position: absolute;
		right: 0;
		top: 0;

		.tool-buttons-wrap {
			box-sizing: border-box;
			position: absolute;
			right: 20px;
			// Note: if {-top == bottom} the box is perfectly
			// centered on the top of the message line.
			top: -$row-pad;
			bottom: $row-pad;
			flex-wrap: nowrap;
		}
	}

	// View raw toggle

	.textBody {
		.textRaw {
			display: none;
		}
		.textRendered {
			display: inherit;
		}
	}

	.textBody.viewRaw {
		.textRaw {
			display: inherit;
		}
		.textRendered {
			display: none;
		}
	}
</style>

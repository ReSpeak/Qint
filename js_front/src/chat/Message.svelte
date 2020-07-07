<script>
	import { hljsHighlight } from './hljs';
	import katex from 'katex';
	import { afterUpdate, onMount } from "svelte";
	import { flash } from "../util";
	import Icon from "../ui/Icon.svelte";

	export let message;

	let viewRaw = false;
	let rendered;
	$: renderedObj = render(message.rendered);

	function render(html) {
		var obj = document.createElement('div');
		obj.innerHTML = html;
		// Apply highlight.js
		for (let elem of obj.getElementsByTagName("code")) {
			hljsHighlight(elem);
		}

		// Apply KaTeX
		for (let elem of obj.getElementsByClassName("latex")) {
			const code = elem.getAttribute('data-latex');
			const mode = elem.getAttribute('data-displaymode');
			try {
				katex.render(code, elem, {
					displayMode: mode === "true",
					throwOnError: false,
				});
			} catch {
				console.error("Failed to render latex");
				elem.innerText = code;
			}
		}
		if (rendered) {
			rendered.innerHTML = '';
			rendered.appendChild(obj);
		}
		return obj;
	}

	let div;
	afterUpdate(() => {
		flash(div);
	});

	onMount(() => {
		rendered.innerHTML = '';
		rendered.appendChild(renderedObj);
	})
</script>

<svelte:options immutable />
<div bind:this={div} class="message-row">
	<div class="message-time">
		<span title="{message.date.format('dddd, MMMM Do YYYY, HH:mm:ss')}">
			{message.date.format('HH:mm')}
		</span>
	</div>
	<!-- msg.status === MessageStatus::Sending -->
	<!-- msg.status === MessageStatus::Error -->
	<div
		class="message-content"
		class:message-sending="{false}"
		class:message-error="{false}"
		class:viewRaw
	>
		<div class="content message-rendered" bind:this={rendered}></div>
		<div class="message-raw">
			<pre>{message.raw}</pre>
		</div>
		<div class="tool-buttons">
			<div class="tool-buttons-wrap buttons has-addons">
				<button class="button is-small is-rounded">
					<Icon name="pencil" />
				</button>
				<button class="button is-small is-rounded">
					<Icon name="format-quote-close" />
				</button>
				<button
					class="button is-small is-rounded"
					on:click="{() => (viewRaw = !viewRaw)}"
				>
					<Icon raw="🥩" />
				</button>
			</div>
		</div>
	</div>
</div>

<style lang="scss">
	.message-row {
		display: contents;

		&:hover > * {
			background-color: mix($background, $text, 90%);

			.tool-buttons {
				visibility: visible;
			}
		}
	}

	.message-time {
		grid-column: 1;
		font-size: 0.8em;
		padding-top: 0.25em;
		* {
			color: mix($text, $background, 60%);
		}
	}

	.message-content {
		grid-column: 2;

		// for tool buttons
		position: relative;

		// Overwrite bulma default
		pre {
			padding: 0;
			border-radius: 7px;

			tab-size: 4;
			-moz-tab-size: 4;
			// TODO Prevent scrollbar
		}
	}

	.message-rendered {
		white-space: pre-wrap;
		word-wrap: break-word;
	}

	.message-rendered :global(pre) {
		background: none;
	}

	.tool-buttons {
		visibility: hidden;
		position: absolute;
		right: 0;
		top: 0;

		.tool-buttons-wrap {
			position: absolute;
			right: 20px;
			top: -10px;
			flex-wrap: nowrap;
		}
	}

	// View raw toggle

	.message-content {
		.message-raw {
			display: none;
		}
		.message-rendered {
			display: inherit;
			// Work-around for tight line-height
			margin-bottom: 0.1em;
		}
	}

	.message-content.viewRaw {
		.message-raw {
			display: inherit;
		}
		.message-rendered {
			display: none;
		}
	}

	.message-raw pre {
		background: none;
	}

	:global(code.hljs) {
		position: relative;
	}

	:global(code[rel]::before) {
		font-size: 0.8em;
		content: attr(rel);
		position: absolute;
		bottom: 0;
		right: 3px;
		color: $orange;
		font-weight: bold;
		font-family: Sans-Serif;
		text-transform: uppercase;
	}
</style>

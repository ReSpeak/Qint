<script lang="typescript">
	import { hljsHighlight } from "./hljs";
	import katex from "katex";
	import { onMount } from "svelte";
	import Icon from "../ui/Icon.svelte";
	import { Message } from "./chat";

	export let message: Message;

	let viewRaw = false;
	let rendered!: HTMLElement;
	let renderedObj!: HTMLElement;
	$: renderedObj = render(message.rendered);

	function render(html: string) {
		var obj = document.createElement("div");
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

<svelte:options immutable />
<div class="message-row">
	<div class="message-time chat-left-col">
		<span title={message.date.format('dddd, MMMM Do YYYY, HH:mm:ss')}>
			{message.date.format('HH:mm')}
		</span>
	</div>
	<!-- msg.status === MessageStatus::Sending -->
	<!-- msg.status === MessageStatus::Error -->
	<div
		class="message-content"
		class:message-sending={false}
		class:message-error={false}
		class:viewRaw>
		<div class="content message-rendered" bind:this={rendered} />
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
				<button class="button is-small is-rounded" on:click={() => (viewRaw = !viewRaw)}>
					<Icon raw="🥩" />
				</button>
			</div>
		</div>
	</div>
</div>

<style lang="scss">
	$row-pad: 0.25em;

	.message-row {
		display: grid;
		grid-template-columns: min-content minmax(0, 1fr);
		padding: $row-pad 0;
		line-height: 1em;

		&:hover {
			background-color: mix($background, $text, 90%);

			.tool-buttons {
				visibility: visible;
			}
		}
	}

	.message-time {
		font-size: 0.8em;
		* {
			color: mix($text, $background, 60%);
		}
	}

	.message-content {
		flex: 1;

		// for tool buttons
		position: relative;

		// Overwrite bulma default
		:global(pre) {
			padding: 0;
			margin: 1em 1em 1em 0;

			tab-size: 4;
			-moz-tab-size: 4;
			// TODO Prevent scrollbar
		}

		.message-raw > pre {
			background: none;
			margin: 0;
		}

		.message-rendered {
			white-space: pre-wrap;
			word-wrap: break-word;
			margin-bottom: 0;

			:global(pre) {
				background: none;
			}
		}
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

	.message-content {
		.message-raw {
			display: none;
		}
		.message-rendered {
			display: inherit;
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

	:global(code.hljs) {
		display: inline-block;
		padding: 0.1em;
	}

	:global(pre code.hljs) {
		display: block;
		padding: 0.5em;

		position: relative;
		border-radius: 7px;
	}

	:global(pre code[rel]::before) {
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

<script lang="typescript">
	import katex from "katex";
	import { hljsHighlight } from "./hljs";
	import { onMount } from "svelte";
	import { Message } from "./chat";
	import { LONG_DATETIME } from "../util";
	import Icon from "../ui/Icon.svelte";
	import LinkPreview from "./LinkPreview.svelte";

	export let unread: boolean;
	export let message: Message;

	let viewRaw = false;
	let rendered!: HTMLElement;
	let links: [string, string][] = [];
	$: renderedObj = render(message.rendered);

	function render(html: string) {
		const obj = document.createElement("div");
		obj.innerHTML = html;

		// Process links and images
		links = [...obj.querySelectorAll("a")]
			.filter((a) => !!a.href)
			.map((a) => [a.href, a.innerText]);

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
<div class="messageRow" class:unread>
	<div class="hover-container">
		<div class="messageTime chat-left-col">
			<span title={message.date.format(LONG_DATETIME)}> {message.date.format('HH:mm')} </span>
		</div>
		<!-- msg.status === MessageStatus::Sending -->
		<!-- msg.status === MessageStatus::Error -->
		<div
			class="messageBody"
			class:message-sending={false}
			class:message-error={false}
			class:viewRaw>
			<div class="messageRendered">
				<div class="content messageTextBody" bind:this={rendered} />
				{#each links as [link, text] (link)}
					<LinkPreview {link} textContent={text} />
				{/each}
			</div>
			<div class="messageRaw">
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
						on:click={() => (viewRaw = !viewRaw)}
						title="It’s raw!">
						<Icon raw="🥩" />
					</button>
				</div>
			</div>
		</div>
	</div>
</div>

<style lang="scss">
	$row-pad: 0.25em;

	.messageRow {
		transition: background 2s;

		&.unread {
			background-color: mix($background, $blue, 80%);
		}
	}

	.hover-container {
		display: grid;
		grid-template-columns: min-content minmax(0, 1fr);
		padding: $row-pad 0;
		line-height: 1.1em;

		&:hover {
			background-color: mix($background, $text, 90%);
			transition: none; // TODO Improve that..., unread should have an animation, hover not

			.tool-buttons {
				visibility: visible;
			}
		}
	}

	.messageTime {
		font-size: 0.8em;
		* {
			color: mix($text, $background, 60%);
		}
	}

	.messageBody {
		flex: 1;

		// for tool buttons
		position: relative;

		// Overwrite bulma default
		:global(pre) {
			position: relative;
			padding: 0;
			margin: 1em 1em 1em 0;
			border-radius: 7px;

			tab-size: 4;
			-moz-tab-size: 4;
			// TODO Prevent scrollbar
		}
	}

	.messageRaw > pre {
		overflow-y: hidden;
		background: none;
		margin: 0;
	}

	.messageTextBody {
		white-space: pre-wrap;
		word-wrap: break-word;
		margin-bottom: 0;

		:global(pre) {
			background: none;
		}
	}

	.messageRendered > *:not(:last-child) {
		padding-bottom: 0.5em;
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

	.messageBody {
		.messageRaw {
			display: none;
		}
		.messageRendered {
			display: inherit;
		}
	}

	.messageBody.viewRaw {
		.messageRaw {
			display: inherit;
		}
		.messageRendered {
			display: none;
		}
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
</style>

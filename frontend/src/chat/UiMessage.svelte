<script lang="typescript">
	import { Message, MessageStatus } from "./chat";
	import { LONG_DATETIME } from "../util";
	import Icon from "../ui/Icon.svelte";
	import LinkPreview from "./LinkPreview.svelte";
	import RenderedText from "../ui/RenderedText.svelte";

	export let unread: boolean;
	export let message: Message;

	let viewRaw = false;
	let links: [string, string][] = [];
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
			class:messageSending={message.status === MessageStatus.Sending}
			class:messageError={message.status === MessageStatus.Error}
			class:isPoke={message.isPoke}
			class:viewRaw>
			<div class="messageRendered">
				<RenderedText text={message.rendered} bind:links />
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
	@import "../style/global_mixin";
	$row-pad: 0.25em;

	.messageRow {
		transition: background 2s;

		&.unread {
			background-color: $highlight-strong;
		}
	}

	.hover-container {
		display: grid;
		grid-template-columns: min-content minmax(0, 1fr);
		padding: $row-pad 0;
		line-height: 1.1em;

		&:hover {
			background-color: $highlight-weak;

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

		&.isPoke {
			display: inline-flex;
		}

		&.isPoke::before {
			content: "Poke: ";
			font-style: italic;
			margin-right: 0.5em;
		}
	}

	.messageRaw > pre {
		overflow-y: hidden;
		background: none;
		margin: 0;
	}

	.messageRendered > :global(*:not(:last-child)) {
		padding-bottom: 0.5em;
	}


	.messageRendered :global(img),
	.messageRendered :global(.chatVideo) {
		//max-height: min(50vh, 30em);
		max-height: min(30em);
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
</style>

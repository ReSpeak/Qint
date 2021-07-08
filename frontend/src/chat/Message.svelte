<svelte:options immutable />

<script lang="ts">
	import { Message, MessageStatus } from "./uiChat";
	import { LONG_DATETIME } from "../util";
	import Icon from "../ui/icon/Icon.svelte";
	import LinkPreview from "./LinkPreview.svelte";
	import RenderedText from "../ui/specialized/RenderedText.svelte";
	import type { NodeSelection } from "../app";
	import type { LinksMap } from "../ui/specialized/uiRenderedText";
	import { DDConnection } from "../connection";

	export let unread: boolean = false;
	export let message: Message;
	export let messageHighlightedContent: string | undefined = undefined;
	export let nodeSel: NodeSelection | undefined = undefined;
	export let server: string | undefined = undefined;
	export let timeFormat: string = "HH:mm";

	let viewRaw = false;
	let links: LinksMap | undefined;
	$: linksArr = links !== undefined ? Array.from(links.values()) : [];
</script>

<div class="messageRow" class:unread>
	<div class="hover-container" style="border-color:{message.clientColor};">
		<div class="messageTime chat-left-col">
			<span title={message.date.format(LONG_DATETIME)}>
				{message.date.format(timeFormat)}
			</span>
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
				<RenderedText
					connection={new DDConnection(nodeSel?.connection, server)}
					text={messageHighlightedContent || message.rendered}
					bind:links />
				{#each linksArr as { link, title } (link)}
					<LinkPreview {link} textContent={title} {nodeSel} />
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
	@import "./chat_style";
	$row-pad: 0.25em;

	.messageRow {
		@extend %unselectable;
		transition: background 2s;

		&.unread {
			background-color: $highlight-strong;
		}
	}

	.hover-container {
		display: grid;
		grid-template-columns: min-content minmax(0, 1fr);
		padding: $row-pad 0;
		border-left: solid $side-pad-width;

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
		// for tool buttons
		position: relative;

		// Overwrite bulma default
		:global(pre) {
			position: relative;
			padding: 0;
			margin: 0 1em 0 0;
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

	.messageRendered {
		:global(.padTop) {
			margin-bottom: 0; // overwrite bulma not-last

			&:not(:first-child) {
				margin-top: 0.5em;
			}
		}

		:global(.limitChatSize) {
			//max-height: min(50vh, 30em);
			max-height: min(30em);
		}
	}

	.messageRendered,
	.messageRaw,
	.messageTime {
		@include textselectable;
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

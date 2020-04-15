<script>
	import { afterUpdate } from "svelte";
	import { flash } from "../util";
	import Icon from "../ui/Icon.svelte";

	export let message;

	let promise;
	let viewRaw = false;

	let div;
	afterUpdate(() => {
		flash(div);
	});

	// Dummy stuff
	let icon;
</script>

<svelte:options immutable />
<div bind:this={div} class="message-row">
	<div class="message-time">
		<span title="{message.date.format('dddd, MMMM Do YYYY, HH:mm:ss')}">
			{message.date.format('HH:mm')}
		</span>
	</div>
	<!-- msg.status == MessageStatus::Sending -->
	<!-- msg.status == MessageStatus::Error -->
	<div
		class="message-content"
		class:message-sending="{false}"
		class:message-error="{false}"
		class:viewRaw
	>
		<div class="content message-rendered latex_proc">{message.text}</div>
		<div class="message-raw">
			<pre>{message.text}</pre>
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
			background-color: #eee;

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
			color: gray;
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
		}
	}

	.message-rendered {
		white-space: pre-wrap;
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
			display: unset;
		}
	}

	.message-content.viewRaw {
		.message-raw {
			display: unset;
		}
		.message-rendered {
			display: none;
		}
	}

	:global(code[rel]::before) {
		font-size: 0.8em;
		content: attr(rel);
		position: absolute;
		bottom: 1em;
		right: 2em;
		color: orange;
		font-weight: bold;
		font-family: Sans-Serif;
		text-transform: uppercase;
	}
</style>

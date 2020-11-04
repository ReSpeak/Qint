<script lang="typescript">
	import { CLIENT_ICON, LONG_DATETIME, SERVER_ICON } from "../util";
	import TsIcon from "../ui/TsIcon.svelte";
	import Icon from "../ui/Icon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import type { SearchResult } from "./search";

	export let content: SearchResult;
</script>

<div class="searchResult">
	<div class="invoker-icon chat-left-col">
		{#if content.message.invoker}
			<!-- TODO Needs server -->
			<!--<TsIcon type="client" source={content.message.invoker} />-->
			<Icon name={CLIENT_ICON} />
		{:else}
			<Icon name={SERVER_ICON} />
		{/if}
	</div>
	<div class="invoker-name has-text-weight-bold">
		{#if content.message.invoker}
			<ClientName client={content.message.invoker} />
		{:else}
			Server
		{/if}
	</div>
	<div class="messageTime chat-left-col">
		<span title={content.message.date.format(LONG_DATETIME)}> {content.message.date.format('HH:mm')} </span>
	</div>
	<div class="messageBody">
		<div class="messageRendered">
			{@html content.highlightedContent}
		</div>
	</div>
</div>

<style lang="scss">
	// TODO Share css
	@import "../style/global_mixin";
	@mixin block-margin {
		margin-top: 0.5em;
	}

	.searchResult {
		display: grid;
		grid-template-columns: min-content minmax(0, 1fr);
		line-height: 1.1em;
		grid-gap: 0.3em;
		padding: 0.3em;

		&:hover {
			background-color: $highlight-weak;
		}
	}

	.messageBody {
		flex: 1;

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

	.messageTime {
		font-size: 0.8em;
		* {
			color: mix($text, $background, 60%);
		}
	}

	.invoker-row {
		display: flex;
		align-items: center;
		@include block-margin;
	}

	.invoker-icon {
		display: flex;
		justify-content: center;
		text-align: center;
	}

</style>

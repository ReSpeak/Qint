<script lang="ts">
	import { SERVER_ICON } from "../util";
	import Icon from "../ui/icon/Icon.svelte";
	import TsIcon from "../ui/icon/TsIcon.svelte";
	import ClientName from "../ui/name/ClientName.svelte";
	import UiMessage from "../chat/Message.svelte";
	import type { MessageSearchResult } from "./uiSearch";

	export let content: MessageSearchResult;
</script>

<div class="searchResult">
	<div class="invoker-icon chat-left-col">
		{#if content.message.invoker}
			<TsIcon type="client" source={content.message.invoker} server={content.server} />
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
	<UiMessage
		unread={false}
		message={content.message}
		server={content.server}
		messageHighlightedContent={content.highlightedContent === null
			? undefined
			: content.highlightedContent} />
</div>

<style lang="scss">
	// TODO Share css
	@import "../style/global_mixin";
	@mixin block-margin {
		margin-top: 0.5em;
	}

	.searchResult {
		/*display: grid;
		grid-template-columns: min-content minmax(0, 1fr);
		line-height: 1.1em;
		grid-gap: 0.3em;*/
		padding: 0.3em;

		&:hover {
			background-color: $highlight-weak;
		}
	}

	.invoker-icon {
		display: flex;
		justify-content: center;
		text-align: center;
	}
</style>

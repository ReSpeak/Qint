<script lang="ts">
	import { SERVER_ICON } from "../util";
	import Icon from "../ui/icon/Icon.svelte";
	import TsIcon from "../ui/icon/TsIcon.svelte";
	import ClientName from "../ui/name/ClientName.svelte";
	import Message from "../chat/Message.svelte";
	import type { MessageSearchResult } from "./uiSearch";
	import { OfflineConnection } from "../connection";

	export let content: MessageSearchResult;
</script>

<div class="searchResult">
	<div class="invoker-row">
		<div class="invoker-icon chat-left-col">
			{#if content.message.invoker}
				<TsIcon
					type="client"
					source={content.message.invoker}
					connection={new OfflineConnection(content.server)} />
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
	</div>
	<Message
		timeFormat="YYYY-MM-DD HH:mm"
		message={content.message}
		connection={new OfflineConnection(content.server)}
		messageHighlightedContent={content.highlightedContent === null
			? undefined
			: content.highlightedContent} />
</div>

<style lang="scss">
	@import "../style/global_mixin";
	@import "../chat/chat_style";

	.invoker-row {
		@include invoker-row;
	}

	.searchResult :global(.chat-left-col) {
		@include chat-left-col;
		width: 80px;
	}

	.searchResult :global(.chat-left-col.messageTime) :global(span) {
		text-align: left;
		padding-left: 0.5em;
	}

	.searchResult {
		padding: 0.3em;

		&:hover {
			background-color: $highlight-weak;
		}
	}
</style>

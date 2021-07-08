<script lang="ts">
	import { CLIENT_ICON } from "../util";
	import Icon from "../ui/icon/Icon.svelte";
	import TsIcon from "../ui/icon/TsIcon.svelte";
	import ClientName from "../ui/name/ClientName.svelte";
	import type { OtherSearchResult } from "./uiSearch";
	import ServerName from "../ui/name/ServerName.svelte";
	import { OfflineConnection } from "../connection";

	export let content: OtherSearchResult;
</script>

<div class="searchResult">
	<div class="result-icon chat-left-col">
		{#if "Channel" in content}
			<TsIcon
				type="channel"
				source={content.Channel.channel}
				connection={new OfflineConnection(content.Channel.server.publicKeyStr)} />
		{:else if "Client" in content}
			<!-- Icon of which server? -->
			<Icon name={CLIENT_ICON} />
		{:else}
			<TsIcon
				type="server"
				source={content.Server.server}
				connection={new OfflineConnection(content.Server.server.publicKeyStr)} />
		{/if}
	</div>
	<div class="resultName has-text-weight-bold">
		{#if "Channel" in content}
			{content.Channel.channel.name}
		{:else if "Client" in content}
			<ClientName client={content.Client.client} />
		{:else}
			<ServerName server={content.Server.server} />
		{/if}
	</div>
	<div class="chat-left-col" />
	<div class="resultBody">
		{#if "Channel" in content}
			<TsIcon
				type="server"
				source={content.Channel.server}
				connection={new OfflineConnection(content.Channel.server.publicKeyStr)} />
			<ServerName server={content.Channel.server} />
			<span class="serverAddress">({content.Channel.server.address})</span>
		{:else if "Server" in content}
			{@html content.Server.highlightedAddress}
		{/if}
	</div>
</div>

<style lang="scss">
	@import "../style/global_mixin";
	@import "../chat/chat_style";

	.searchResult :global(.chat-left-col) {
		@include chat-left-col;
		width: 40px;
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

	.resultName {
		align-self: center;
	}

	.resultBody {
		display: flex;
		align-items: center;
		gap: 0.5em;
	}
</style>

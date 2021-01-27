<script lang="typescript">
	import { CLIENT_ICON, LONG_DATETIME, SERVER_ICON } from "../util";
	import Icon from "../ui/Icon.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import type { OtherSearchResult } from "./search";
	import ServerName from "../ui/ServerName.svelte";

	export let content: OtherSearchResult;
</script>

<div class="searchResult">
	<div class="result-icon chat-left-col">
		{#if "Channel" in content}
			<TsIcon type="channel" source={content.Channel.channel} server={content.Channel.server.publicKeyStr} />
		{:else if "Client" in content}
			<Icon name={CLIENT_ICON} />
		{:else}
			<Icon name={SERVER_ICON} />
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
	<div class="chat-left-col">
	</div>
	<div class="resultBody">
		{#if "Channel" in content}
			<TsIcon type="server" source={content.Channel.server} server={content.Channel.server.publicKeyStr} />
			<ServerName server={content.Channel.server} />
			<span class="serverAddress">({content.Channel.server.address})</span>
		{:else if "Server" in content}
			{content.Server.server.address}
		{/if}
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

	.resultName {
		align-self: center;
	}

	.resultBody {
		display: flex;
		align-items: center;
		gap: 0.5em;
	}
</style>

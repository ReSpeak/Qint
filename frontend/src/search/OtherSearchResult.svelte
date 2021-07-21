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
	<div class="result-icon">
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
	<div class="resultName">
		{#if "Channel" in content}
			{@html content.Channel.highlightedName} on
			<ServerName server={content.Channel.server} />
		{:else if "Client" in content}
			<ClientName client={content.Client.client} />
		{:else}
			<ServerName server={content.Server.server} />
			{@html content.Server.highlightedAddress}
		{/if}
	</div>
</div>

<style lang="scss">
	@import "../style/global_mixin";

	.searchResult {
		display: grid;
		grid-template-columns: min-content minmax(0, 1fr);
		line-height: 1.1em;
		grid-gap: 0.1em;
		padding: 0.3em;
	}

	.resultName {
		align-self: center;
	}
</style>

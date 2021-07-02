<script lang="ts">
	import { Connection } from "../connection";
	import ServerName from "../ui/name/ServerName.svelte";
	import RenderedText from "../ui/specialized/RenderedText.svelte";
	import ConnectionSettings from "../bar/ConnectionSettings.svelte";

	export let connection: Connection;
	$: serverRaw = connection.book.server;
	$: server = $serverRaw;
</script>

<div class="name">
	<ServerName {server} />
</div>
{#if server.hostmessageRendered && server.hostmessageRendered.length > 0}
	<div class="description">
		<RenderedText {connection} text={server.hostmessageRendered} />
	</div>
{/if}
{#if server.welcomeMessageRendered && server.welcomeMessageRendered.length > 0}
	<div class="description">
		<RenderedText {connection} text={server.welcomeMessageRendered} />
	</div>
{/if}
<ConnectionSettings {connection} />

<style lang="scss">
</style>

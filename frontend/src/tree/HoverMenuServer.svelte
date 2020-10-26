<script lang="typescript">
	import { Connection } from "../connection";
	import ServerName from "../ui/ServerName.svelte";
	import RenderedText from "../ui/RenderedText.svelte";
	import ConnectionSettings from "../bar/ConnectionSettings.svelte";

	export let connection: Connection;
	$: serverRaw = connection.book.server;
	$: server = $serverRaw;
</script>

<div class="name">
	<ServerName {connection} />
</div>
{#if server.hostmessageRendered.length > 0}
<div class="description">
	<RenderedText text={server.hostmessageRendered} />
</div>
{/if}
{#if server.welcomeMessageRendered.length > 0}
<div class="description">
	<RenderedText text={server.welcomeMessageRendered} />
</div>
{/if}
<ConnectionSettings {connection} />

<style lang="scss">
</style>

<script lang="typescript">
	import { Connection } from "../connection";
	import ServerName from "../ui/ServerName.svelte";

	export let connection: Connection;
	$: serverRaw = connection.book.server;
	$: server = $serverRaw;
</script>

<div class="name">
	<ServerName {connection} />
</div>
{#if server.hostmessage.length > 0}
<div>
	{server.hostmessage}
</div>
{/if}
{#if server.welcomeMessage.length > 0}
<div>
	{server.welcomeMessage}
</div>
{/if}

<style lang="scss">
	* {
		font-weight: normal;
	}
</style>

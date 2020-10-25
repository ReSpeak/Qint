<script lang="typescript">
	import { Connection } from "../connection";
	import type { ClientId } from "../ts";
	import ClientName from "../ui/ClientName.svelte";
	import ClientVolume from "../ui/ClientVolume.svelte";

	export let connection: Connection;
	export let clientId: ClientId;

	$: clientRaw = connection.book.clients.get(clientId)!;
	$: client = $clientRaw;
	$: ownClient = clientId === connection.book.ownClientId;
</script>

<div class="name">
	<ClientName {client} />
	{#if client.awayMessage !== null && client.awayMessage.length !== 0}
		({client.awayMessage})
	{/if}
</div>
{#if client.description.length > 0}
<div>
	{client.description}
</div>
{/if}
{#if !ownClient}
	<ClientVolume {client} {connection} />
{/if}

<style lang="scss">
</style>

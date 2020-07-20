<script lang="typescript">
	import { Connection } from "../connection";
	import { Client } from "../tree/book";
	import { Moment } from "moment";
	import PlatformIcon from "../ui/PlatformIcon.svelte";
	import ClientIcon from "../ui/ClientIcon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import { getClientAvatarPath } from "../ui/clientIcon";

	export let connection!: Connection;
	export let clientId!: number;

	let client: Client;
	let avatarPath: string | undefined;
	let onlineSince: Moment;
	$: {
		client = connection.book.getClient(clientId)!;
		avatarPath = getClientAvatarPath(client, connection);
	}
</script>

<div class="descGroup">
	<div class="dataLine headLine">
		<ClientIcon {client} {connection} />
		<ClientName {client} />
		<div style="flex: 1;" ></div>
		<div>
			{"Version"}
			<PlatformIcon platform={"Platform"} />
		</div>
	</div>
	<div class="dataLine">
		<div>Description:</div>
		<div>{client.description}</div>
	</div>
	<div class="dataLine">
		<div>Online since:</div>
		<div>No Data</div>
	</div>
	<div class="dataLine">
		<div>Time away:</div>
		<div>No Data</div>
	</div>
</div>
{#if avatarPath}
	<img class="clientAvatar" src={avatarPath} alt="Client avatar" />
{/if}

<style>
	.clientAvatar {
		max-width: 100%;
	}
</style>

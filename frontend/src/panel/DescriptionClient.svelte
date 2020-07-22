<script lang="typescript">
	import { Connection } from "../connection";
	import { Client } from "../tree/book";
	import { Moment } from "moment";
	import Icon from "../ui/Icon.svelte";
	import PlatformIcon from "../ui/PlatformIcon.svelte";
	import ClientIcon from "../ui/ClientIcon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import ClientVolume from "../controls/ClientVolume.svelte";
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

<StickyList>
	<StickySlot>Info</StickySlot>
	<div class="descGroup">
		<div class="dataLine headLine">
			<ClientIcon {client} {connection} />
			<ClientName {client} />
			<div style="flex: 1;" />
			<div>
				{'Version'}
				<PlatformIcon platform={'Platform'} />
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
		{#if avatarPath}
			<img class="clientAvatar" src={avatarPath} alt="Client avatar" />
		{/if}
	</div>
	<StickySlot>Actions</StickySlot>
	<div class="descGroup">
		<p class="buttons">
			<button class="button is-small is-warning">
				<Icon name="shoe-formal" />
				<span>Kick Channel</span>
			</button>
			<button class="button is-small is-danger">
				<Icon name="shoe-formal" />
				<span>Kick Server</span>
			</button>
			<button class="button is-small is-danger">
				<Icon name="cancel" />
				<span>Ban</span>
			</button>
		</p>
		<div class="dataLine">
			<div>Volume:</div>
			<ClientVolume {client} {connection} />
		</div>
	</div>
</StickyList>

<style>
	.clientAvatar {
		max-width: 100%;
	}
</style>

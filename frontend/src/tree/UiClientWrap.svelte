<svelte:options immutable={true}/>

<script lang="typescript">
	import { Channel, Client } from "../book";
	import { Connection } from "../connection";
	import UiClient from "./UiClient.svelte";

	export let connection: Connection;
	export let client: Client;
	export let filter: string;
	export let channel: Channel | undefined = undefined;
	export let filterShow: boolean = true;
</script>

<UiClient {connection} {client} {channel} {filter} bind:filterShow={filterShow} />

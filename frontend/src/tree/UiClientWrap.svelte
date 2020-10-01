<svelte:options immutable={true}/>

<script lang="typescript">
	import { Client } from "../book";
	import { Connection } from "../connection";
	import UiClient from "./UiClient.svelte";

	export let connection: Connection;
	export let client: Client;
	export let filter: string;
	export let filterShow: boolean = true;
</script>

<UiClient {connection} {client} {filter} bind:filterShow={filterShow} />

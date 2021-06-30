<svelte:options immutable />

<script lang="ts">
	import UiServer from "./Server.svelte";
	import type { Connection } from "../connection";
	import { ConnectData } from "../connect/uiConnect";

	export let connection: Connection;
	export let filter: string;
	export let showConnect: (data: ConnectData) => void;
</script>

<UiServer {connection} {filter} {showConnect} />

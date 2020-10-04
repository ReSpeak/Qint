<svelte:options immutable />
<script lang="typescript">
	import UiServer from "./UiServer.svelte";
	import type { Connection } from "../connection";
	import { ConnectData } from "../connect/connect";

	export let connection: Connection;
	export let filter: string;
	export let showConnect: (data: ConnectData) => void;
</script>

<UiServer {connection} {filter} {showConnect} />

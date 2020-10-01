<svelte:options immutable />
<script lang="typescript">
	import UiServer from "./UiServer.svelte";
	import type { Connection } from "../connection";

	export let connection: Connection;
	export let filter: string;
</script>

<UiServer {connection} {filter} />

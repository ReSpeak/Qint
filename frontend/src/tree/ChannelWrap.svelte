<svelte:options immutable={true} />

<script lang="ts">
	import { Channel } from "../book";
	import { Connection } from "../connection";
	import UiChannel from "./Channel.svelte";

	export let connection: Connection | undefined = undefined;
	export let server: string | undefined = undefined;
	export let filter: string;
	export let filterShow: boolean = true;
	export let filterStartFromRoot: boolean;
	export let channel: Channel;
</script>

<UiChannel {connection} {server} {filter} bind:filterShow {filterStartFromRoot} {channel} />

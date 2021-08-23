<svelte:options immutable={true} />

<script lang="ts">
	import { Channel } from "../book";
	import type { IConnection } from "../connection";
	import UiChannel from "./Channel.svelte";

	export let connection: IConnection;
	export let filter: string;
	export let filterShow: boolean = true;
	export let filterStartFromRoot: boolean;
	export let channel: Channel;
</script>

<UiChannel {connection} {filter} bind:filterShow {filterStartFromRoot} {channel} />

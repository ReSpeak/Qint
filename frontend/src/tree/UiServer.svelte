<script lang="typescript">
	import UiChannel from "./UiChannelWrap.svelte";
	import { Connection } from "../connection";
	import { flash } from "../util";
	import { afterUpdate } from "svelte";

	let div: HTMLElement;
	afterUpdate(() => flash(div));

	export let connection: Connection;
	export let filter: string;
	let book = connection.book;
	let channels = book.server.channels;
	$: filterStartFromRoot = filter.includes("/");
</script>

<div class="menu channel-list">
	<ul class="menu-list">
		{#each $channels as channel (channel.id)}
			<UiChannel {connection} {filter} {filterStartFromRoot} {channel} />
		{/each}
	</ul>
</div>

<style lang="scss">
	ul {
		margin: 0 0 0 0.2em;
	}

	:global(.innerContainer.dragStyle) {
		background-color: #6040c080 !important;
		z-index: 3 !important;
	}
</style>

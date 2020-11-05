<script lang="typescript">
	import type { Writable } from "svelte/store";
	import UiServer from "../tree/UiServerWrap.svelte";
	import UiSearch from "../search/UiSearch.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import { Connection } from "../connection";
	import { ConnectData } from "../connect/connect";

	export let connections: Writable<Connection[]>;
	export let filter: string;
	export let visible: boolean;
	export let showConnect: (data: ConnectData) => void;
</script>

<aside class="sidebar" class:hidden={!visible}>
	<StickyList>
		{#each $connections as connection (connection.backend.id)}
			<UiServer {connection} {filter} {showConnect} />
		{/each}

		<StickySlot>Notifications</StickySlot>
		<UiSearch {filter} />
	</StickyList>
</aside>

<style lang="scss">
	.sidebar {
		display: inline-flex;
		flex-direction: column;
		background-color: $box-background-color;
		box-shadow: 3px 0 3px #0006;
		overflow-y: auto;
		z-index: 3; // Required to be over the chat
	}

	.sidebar > .menu .menu-list li {
		margin: 2em;
	}
</style>

<script lang="typescript">
	import { writable } from "svelte/store";
	import UiChat from "./chat/UiChat.svelte";
	import Searchbar from "./bar/Searchbar.svelte";
	import Sidebar from "./bar/Sidebar.svelte";
	import Toolbar from "./bar/Toolbar.svelte";
	import { Connection } from "./connection";

	export let connection: Connection;
	let filter;

	let showSidebar = writable(true);
	let showChat = writable(true);
	let columnStyle = "";

	$: {
		columnStyle = "";
		if ($showSidebar)
			columnStyle += " var(--channel-tree-width)";
		else
			columnStyle += " 0";
		if ($showChat)
			columnStyle += " 1fr";
	}
</script>

<div class="connected-container" style="grid-template-columns: {columnStyle}">
	<Toolbar {connection} {showSidebar} {showChat} />
	{#if $showSidebar}
		<Searchbar bind:filter/>
		<Sidebar {connection} {filter}/>
	{/if}
	{#if $showChat}
		<UIChat {connection}/>
	{/if}
</div>

<style lang="scss">
	.connected-container {
		display: grid;

		position: absolute;
		top: 0;
		right: 0;
		bottom: 0;
		left: 0;
	}

	.connected-container > :global(.toolbar) {
		grid-row: 1;
		grid-column: 2;
	}

	.connected-container > :global(.searchbar) {
		grid-row: 1;
		grid-column: 1;
	}

	.connected-container > :global(.sidebar) {
		grid-row: 2;
		grid-column: 1;
	}

	.connected-container > :global(.chat) {
		grid-row: 2;
		grid-column: 2;
	}
</style>

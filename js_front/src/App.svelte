<script>
	import { onMount, onDestroy } from "svelte";
	import { writable } from "svelte/store";
	import Connect from "./connect/Connect.svelte";
	import Connected from "./Connected.svelte";
	import { ConnectionState } from "./connection";

	export let connection;
	$: state = connection.state;
</script>

{#if $state !== ConnectionState.Connected && $state !== ConnectionState.ChannelListFinished}
	<Connect {connection} />
{:else}
	<Connected {connection} />
{/if}

<style lang="scss" global>
	@import "@mdi/font/css/materialdesignicons";
	@import "katex/dist/katex.min";
	@import "highlight.js/styles/vs2015";

	:root {
		--channel-tree-width: 20em;
	}
	* {
		margin: 0;
	}
	html {
		overflow: auto;
		background-color: $background;
	}


	.hover.menu {
		position: fixed;
		z-index: 3;
		border: solid 1px $border;
		border-radius: 0.5em;
		background: $background;
		padding: 0.5em;
	}

	.hover.menu .corner {
		position: absolute;
		transform: rotate(45deg);
		left: -0.3em;
		top: 0.8em;
		width: 0.5em;
		height: 0.5em;
		border-left: solid 1px $border;
		border-bottom: solid 1px $border;
		background: $background;
	}
</style>

<script>
	import { onMount, onDestroy } from "svelte";
	import { writable } from "svelte/store";
	import Connect from "./connect/Connect.svelte";
	import Connected from "./Connected.svelte";
	import { ConnectionState } from "./connection";

	export let connection;
	$: state = connection.state;
</script>

{#if $state !== ConnectionState.Connected}
	<Connect {connection} />
{:else}
	<Connected {connection} />
{/if}

<style lang="scss" global>
	@import "@mdi/font/css/materialdesignicons";
	@import "katex/dist/katex.min.css";
	@import "highlight.js/styles/vs2015.css";

	:global(:root) {
		--channel-tree-width: 20em;
	}
	:global(*) {
		margin: 0;

	}
	:global(html) {
		overflow: auto;
	}
	:global(body) {
		background: white;
		color: black;
	}
	:global(input::placeholder) {
		color: black;
	}



	:global(.hover.menu) {
		position: fixed;
		z-index: 3;
		border: solid 1px #333;
		border-radius: 0.5em;
		background: white;
		padding: 0.5em;
	}

	:global(.hover.menu .corner) {
		position: absolute;
		transform: rotate(45deg);
		left: -0.3em;
		top: 0.8em;
		width: 0.5em;
		height: 0.5em;
		border-left: solid 1px #333;
		border-bottom: solid 1px #333;
		background: white;
	}
</style>

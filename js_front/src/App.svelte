<script>
	import { onMount, onDestroy } from "svelte";
	import { writable } from "svelte/store";
	import Connect from "./Connect.svelte";
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

<style lang="scss">
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
</style>

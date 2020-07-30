<script lang="ts">
	import Icon from "./Icon.svelte";
	import { Client } from "../tree/book";
	import { Connection } from "../connection";
	import { getClientIconPath } from "./clientIcon";

	export let client: Client;
	// Either connection or server has to be set to fetch the icon
	export let connection: Connection | undefined = undefined;
	export let server: string | undefined = undefined;

	$: iconPath = getClientIconPath(client, connection, server);
</script>

{#if iconPath}
	<span class="icon">
		<img src={iconPath} alt="Client icon" />
	</span>
{:else}
	<Icon name="account" />
{/if}

<style>
	img {
		object-fit: scale-down;
		height: 1.5em;
		width: 1.5em;
	}
</style>

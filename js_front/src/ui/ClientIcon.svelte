<script>
	import Icon from './Icon.svelte';
	import getClientIconPath from "./clientIcon";

	export let client;
	// Either connection or server has to be set to fetch the icon
	export let connection = undefined;
	export let server = undefined;

	$: clientIconPath = getClientIconPath(client, connection, server);
</script>

{#if clientIconPath}
	<span class="icon">
		<img src={clientIconPath} alt="" />
	</span>
{:else}
	<Icon name="account" />
{/if}

<style>
</style>

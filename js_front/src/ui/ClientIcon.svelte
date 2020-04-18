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
	<img src={clientIconPath} alt="" />
{:else}
	<Icon name="account" />
{/if}

<style lang="scss">
	img {
		object-fit: scale-down;
		height: 1.5em;
		width: 1.5em;
	}
</style>
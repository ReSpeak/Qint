<script>
	import Icon from './Icon.svelte';
	import getClientIconPath from "./clientIcon";

	// Either connection or server has to be set to fetch the icon
	export let connection = undefined;
	export let server = undefined;
	let conServer = connection.book.server;

	// I’m sorry for using javascript
	$: serverIconPath = getClientIconPath($conServer, connection, server);
</script>

{#if serverIconPath}
	<span class="icon">
		<img src={serverIconPath} alt="" />
	</span>
{:else}
	<Icon name="server" />
{/if}

<style lang="scss">
	img {
		object-fit: scale-down;
		height: 1.5em;
		width: 1.5em;
	}
</style>
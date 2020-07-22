<script lang="typescript">
	import Icon from "./Icon.svelte";
	import { Connection } from "../connection";
	import { getIconPath } from "./clientIcon";

	// Either connection or server has to be set to fetch the icon
	export let connection!: Connection;
	export let server: string | undefined = undefined;
	let conServer = connection !== undefined ? connection.book.server : undefined;
	declare let iconPath: string | undefined;
	$: iconPath = getIconPath($conServer, connection, server);
</script>

{#if iconPath}
	<span class="icon">
		<img src={iconPath} alt="" />
	</span>
{:else}
	<Icon name="server" />
{/if}

<style>
	img {
		object-fit: scale-down;
		height: 1.5em;
		width: 1.5em;
	}
</style>

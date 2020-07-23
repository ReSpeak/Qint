<script lang="typescript">
	import Icon from './Icon.svelte';
	import { Channel } from "../tree/book";
	import { Connection } from "../connection";
	import { getClientIconPath } from "./clientIcon";

	export let channel: Channel;
	// Either connection or server has to be set to fetch the icon
	export let connection: Connection | undefined = undefined;
	export let server: string | undefined = undefined;

	let iconPath: string | undefined;
	$: iconPath = getClientIconPath(channel, connection, server);
</script>

{#if iconPath}
	<span class="icon">
		<img src={iconPath} alt="" />
	</span>
{:else}
	<Icon name="chat-outline" />
{/if}

<style>
	img {
		object-fit: scale-down;
		height: 1.5em;
		width: 1.5em;
	}
</style>

<script>
	import Icon from './Icon.svelte';
	import getClientIconPath from "./clientIcon";

	export let channel;
	// Either connection or server has to be set to fetch the icon
	export let connection = undefined;
	export let server = undefined;

	// I’m sorry for using javascript
	$: channelIconPath = getClientIconPath(channel, connection, server);
</script>

{#if channelIconPath}
	<img src={channelIconPath} alt="" />
{:else}
	<Icon name="chat-outline" />
{/if}

<style lang="scss">
	img {
		object-fit: scale-down;
		height: 1.5em;
		width: 1.5em;
	}
</style>
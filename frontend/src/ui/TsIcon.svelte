<script lang="typescript">
	import Icon from "./Icon.svelte";
	import { Connection } from "../connection";
	import { getClientIconPath } from "./clientIcon";
	import type { IconSourceLike } from "./clientIcon";

	// Either connection or server has to be set to fetch the icon
	export let connection: Connection | undefined = undefined;
	export let server: string | undefined = undefined;
	export let source: IconSourceLike;
	export let type: "server" | "channel" | "client";

	let fallback: string;
	switch (type) {
		case "server":
			fallback = "server";
			break;
		case "channel":
			fallback = "chat-outline";
			break;
		case "client":
			fallback = "account";
			break;
		default:
			fallback = "progress-question";
			break;
	}

	$: iconPath = getClientIconPath(source, connection, server);
</script>

{#if iconPath}
	<span class="icon">
		<img src={iconPath} alt="{type} icon" />
	</span>
{:else if fallback}
	<Icon name={fallback} />
{/if}

<style>
	img {
		object-fit: scale-down;
		height: 1.5em;
		width: 1.5em;
		/* If the icon is not found and the alt text is displayed */
		overflow: hidden;
	}
</style>

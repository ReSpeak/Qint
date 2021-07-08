<script lang="ts">
	import Icon from "./Icon.svelte";
	import type { IConnection } from "../../connection";
	import { getClientIconPath } from "./tsIcons";
	import type { IconSourceLike } from "./tsIcons";

	export let connection: IConnection | undefined;
	export let source: IconSourceLike | null | undefined;
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

	let iconPath: string | undefined;
	$: {
		if (connection)
			getClientIconPath(connection, source).then(path => iconPath = path);
		else
			iconPath = undefined;
	}
</script>

{#if iconPath}
	<span class="icon">
		<img src={iconPath} alt="{type} icon" on:error={() => (iconPath = undefined)} />
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

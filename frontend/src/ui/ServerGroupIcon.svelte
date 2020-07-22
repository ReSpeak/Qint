<script lang="typescript">
	import { get, Writable } from "svelte/store";
	import Icon from "./Icon.svelte";
	import { Connection } from "../connection";
	import { ServerGroup } from "../tree/book";
	import { getIconPath, DummyStore } from "./clientIcon";

	export let id!: number;
	// Either connection or server has to be set to fetch the icon
	export let connection!: Connection;
	export let server: string | undefined = undefined;

	let seg: Writable<ServerGroup>;
	let iconPath: string | undefined;
	const sgs = connection.book.serverGroups;
	$: seg = $sgs.get(id) ?? DummyStore;
	$: iconPath = getIconPath($seg, connection, server);
</script>

{#if iconPath}
	{#if iconPath.startsWith("alpha")}
		<Icon name={iconPath} />
	{:else}
		<span class="icon">
			<img src={iconPath} alt="" />
		</span>
	{/if}
{/if}

<style>
	img {
		object-fit: scale-down;
		height: 1.5em;
		width: 1.5em;
	}
</style>

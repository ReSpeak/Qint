<script lang="typescript">
	import type { Writable } from "svelte/store";
	import Icon from "./Icon.svelte";
	import { Connection } from "../connection";
	import { getIconPath, DummyStore } from "./clientIcon";
	import type { IconSource } from "./clientIcon";

	export let id: number;
	// Either connection or server has to be set to fetch the icon
	export let connection: Connection;
	export let server: string | undefined = undefined;

	let seg: Writable<IconSource>;
	let iconPath: string;
	const sgs = connection.book.serverGroups;
	$: seg = $sgs.get(id) ?? DummyStore;
	$: iconPath = getIconPath($seg, connection, server) ?? "";
</script>

{#if iconPath}
	{#if iconPath.startsWith('alpha')}
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

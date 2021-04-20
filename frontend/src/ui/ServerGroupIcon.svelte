<script lang="ts">
	import Icon from "./Icon.svelte";
	import { Connection } from "../connection";
	import { getIconPath, DummyStore } from "./clientIcon";
	import { ServerGroup } from "../book";
	import type { ServerGroupId } from "../ts";

	export let id: ServerGroupId;
	// Either connection or server has to be set to fetch the icon
	export let connection: Connection;
	export let server: string | undefined = undefined;

	const sgs = connection.book.serverGroups;
	$: seg = $sgs.get(id) ?? DummyStore;
	$: iconPath = getIconPath($seg, connection, server) ?? "";
	let name: string | undefined;
	$: {
		const group = $seg;
		if (group && group instanceof ServerGroup) {
			name = group.name;
		} else {
			name = undefined;
		}
	}
</script>

{#if iconPath}
	<span title={name} class="serverGroupIcon">
		{#if iconPath.startsWith("alpha")}
			<Icon name={iconPath} />
		{:else}
			<span class="icon">
				<img src={iconPath} alt="" />
			</span>
		{/if}
	</span>
{/if}

<style>
	img {
		object-fit: scale-down;
		height: 1.5em;
		width: 1.5em;
	}

	.serverGroupIcon {
		/* Otherwise the hight is more than it needs to be */
		display: flex;
	}
</style>

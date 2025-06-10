<script lang="ts">
	import Icon from "./Icon.svelte";
	import type { Connection } from "../../connection";
	import { getIconPath, DummyStore } from "./tsIcons";
	import { ServerGroup } from "../../book";
	import type { ServerGroupId } from "../../ts";

	export let id: ServerGroupId;
	export let connection: Connection;

	const sgs = connection.book.serverGroups;
	$: seg = $sgs.get(id) ?? DummyStore;
	$: iconPromise = getIconPath(connection, $seg) ?? "";
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

{#await iconPromise then iconPath}
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
{/await}

<style>
	img {
		object-fit: scale-down;
		height: 1.5em;
		width: 1.5em;
	}

	.serverGroupIcon {
		/* Otherwise the height is more than it needs to be */
		display: flex;
	}
</style>

<svelte:options immutable={true} />

<script lang="ts">
	import { getContext, onDestroy } from "svelte";
	import { contextKey } from "./uiTabList";
	import type { TabListContext } from "./uiTabList";
	import { assert } from "../../util";

	export let title: string;
	export let selected: boolean = false;

	const context: TabListContext = getContext(contextKey);
	assert(context !== undefined, "TabSlot must be used within a TabList");
	const ownId = context.registerPanel(title);
	const activeId = context.activeId;
	$: selected = $activeId === ownId;

	onDestroy(() => context.unregisterPanel(ownId));
</script>

{#if selected}
	<slot />
{/if}

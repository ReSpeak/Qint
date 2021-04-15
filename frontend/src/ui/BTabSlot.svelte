<script lang="ts">
	import { getContext, onDestroy } from "svelte";
	import { contextKey } from "./tabList";
	import type { TabListContext } from "./tabList";
	import { assert } from "../util";

	export let title: string;
	export let selected: boolean = false;

	let context: TabListContext = getContext(contextKey);
	assert(context !== undefined, "TabSlot must be used within a TabList");
	const ownId = context.registerPanel(title);
	const activeId = context.activeId;
	$: selected = $activeId === ownId;

	onDestroy(() => context.unregisterPanel(ownId));
</script>

<svelte:options immutable="{true}" />
{#if selected}
	<slot />
{/if}

<script lang="typescript">
	import { getContext } from "svelte";
	import { TabListContext, contextKey } from "./bTabList";
	import { assert } from "../util";

	export let title!: string;

	let context: TabListContext = getContext(contextKey);
	assert(context !== undefined, "TabSlot must be used within a TabList");
	const ownIndex = context.registerPanel(title);
	const activeIndex = context.activeIndex;
</script>

<svelte:options immutable="{true}" />
{#if $activeIndex === ownIndex}
	<slot />
{/if}

<script lang="typescript">
	import { assert } from "../util";
	export let items: any[];
	export let selected: any;
	export let display: (item: any) => string = displayFn;
	let selectedIndex!: number;
	assert(Array.isArray(items), "items must be set to an array");

	function displayFn(item: any): string {
		if (typeof item === "string") {
			return item;
		} else if ("text" in item) {
			return item.text;
		} else {
			return String(item);
		}
	}
</script>

<svelte:options immutable="{true}" />
<div class="select is-fullwidth">
	<select bind:value="{selectedIndex}" on:change="{() => (selected = items[selectedIndex])}">
		{#each items as item, index}
			<option value="{index}">{display(item)}</option>
		{/each}
	</select>
</div>

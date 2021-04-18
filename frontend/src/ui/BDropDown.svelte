<script lang="ts">
	import { hasProperty } from "../util";
	import { createEventDispatcher, onMount } from "svelte";
	type DDObjElement = { value: any };
	export let items: readonly unknown[];
	export let selected: unknown = undefined;
	export let display: (item: any) => string = displayFn;
	export let id: string | undefined = undefined;
	let dd: HTMLSelectElement;
	const dispatch = createEventDispatcher<{ change: any }>();

	$: selectedToIndex(selected);

	function selectedToIndex(selected: any) {
		if (dd == null || items.length === 0) return;
		if (hasProperty(items[0], "value")) {
			const index = (items as DDObjElement[]).findIndex(it => it.value === selected);
			if (index === -1) return;
			dd.selectedIndex = index;
		} else {
			const newIndex = (items as string[]).findIndex(i => i === selected);
			if (newIndex !== -1) {
				dd.selectedIndex = newIndex;
			}
		}
	}

	function indexToSelected() {
		if (dd == null || dd.selectedIndex >= items.length) return;
		const pickedItem = items[dd.selectedIndex];
		if (hasProperty(pickedItem, "value")) {
			selected = (pickedItem as DDObjElement).value;
		} else {
			selected = pickedItem;
		}
		dispatch("change", selected);
	}

	function displayFn(item: any): string {
		if (typeof item === "string") {
			return item;
		} else if ("text" in item) {
			return item.text;
		} else {
			return String(item);
		}
	}

	onMount(() => {
		selectedToIndex(selected);
	});
</script>

<svelte:options immutable={true} />
<div class="select is-fullwidth">
	<!-- svelte-ignore a11y-no-onchange -->
	<select bind:this={dd} on:change={indexToSelected} id={id}>
		{#each items as item, index}
			<option value={index}>{display(item)}</option>
		{/each}
	</select>
</div>

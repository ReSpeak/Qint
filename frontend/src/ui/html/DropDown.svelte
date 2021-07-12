<svelte:options immutable={true} />

<script lang="ts">
	import { createEventDispatcher, onMount } from "svelte";

	type T = $$Generic;

	export let items: ReadonlyArray<T>;
	export let selected: T | undefined = undefined;
	export let display: (item: T) => string = displayFn;
	export let compare: (a: T, sel: T | undefined) => boolean = compareFn;
	export let id: string | undefined = undefined;
	let dd: HTMLSelectElement;
	const dispatch = createEventDispatcher<{ change: T }>();

	$: selectedToIndex(selected);

	function selectedToIndex(selected: T | undefined) {
		if (dd == null || items.length === 0) return;
		const newIndex = items.findIndex((i) => compare(i, selected));
		if (newIndex !== -1) {
			dd.selectedIndex = newIndex;
		}
	}

	function indexToSelected() {
		if (dd == null || dd.selectedIndex >= items.length) return;
		const pickedItem = items[dd.selectedIndex];
		selected = pickedItem;
		dispatch("change", selected);
	}

	function displayFn(item: T): string {
		if (typeof item === "string") {
			return item;
		} else {
			return String(item);
		}
	}

	function compareFn(a: T, sel: T | undefined): boolean {
		return a === sel;
	}

	onMount(() => {
		selectedToIndex(selected);
	});
</script>

<div class="select is-fullwidth">
	<!-- svelte-ignore a11y-no-onchange -->
	<select bind:this={dd} on:change={indexToSelected} {id}>
		{#each items as item, index}
			<option value={index}>{display(item)}</option>
		{/each}
	</select>
</div>

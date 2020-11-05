<script lang="typescript">
	import { createEventDispatcher, onMount } from "svelte";
	export let items: any[];
	export let selected: any;
	export let display: (item: any) => string = displayFn;
	export let id: string | undefined = undefined;
	let dd: HTMLSelectElement;
	const dispatch = createEventDispatcher<{ change: any }>();

	$: selectedToIndex(selected);

	function selectedToIndex(selected: any) {
		if (dd == null) return;
		const newIndex = items.findIndex(i => i === selected);
		if (newIndex !== -1) {
			dd.selectedIndex = newIndex;
		}
	}

	function indexToSelected() {
		if (dd == null) return;
		selected = items[dd.selectedIndex];
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

<script lang="ts">
	import { onMount } from "svelte";
	import { getContext } from 'svelte';

	export let label: string;
	export let narrow: boolean = false;
	export let title: string = "";
	export let labelStyle: string = "";

	const ctx = getContext("component_id") as any ?? "";
	const labelId: string = label.replace(/\s/g, '-') + ctx;
	let slot: HTMLElement | undefined;

	onMount(() => {
		if (slot) {
			let inputField = slot.querySelector("input");
			if (inputField) {
				inputField.id = labelId;
			}
			slot = undefined;
		}
	});
</script>

<svelte:options immutable="{true}" />
<!-- svelte-ignore a11y-label-has-associated-control -->
<div class="field is-horizontal" title={title}>
	<div class="field-label {labelStyle}">
		<label class="label" for={labelId}>{label}</label>
	</div>
	<div class="field-body">
		<div bind:this={slot} class="field" class:is-narrow="{narrow}">
			<slot />
		</div>
	</div>
</div>

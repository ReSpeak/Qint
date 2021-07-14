<script lang="ts">
	import { createEventDispatcher } from "svelte";
	import Icon from "../icon/Icon.svelte";

	export let disabled = false;

	const dispatch = createEventDispatcher<{
		delete: undefined;
	}>();

	let isDeleting = false;

	function deleteConfirmed() {
		isDeleting = false;
		dispatch("delete");
	}
</script>

<div class="field has-addons">
	{#if isDeleting}
		<p class="control">
			<button class="button" on:click={() => (isDeleting = false)}>
				<Icon name="close" />
			</button>
		</p>
		<p class="control">
			<button class="button is-danger" on:click={deleteConfirmed}>
				<Icon name="delete-alert" />
			</button>
		</p>
	{:else}
		<p class="control">
			<button
				{disabled}
				class="button is-danger is-outlined"
				on:click={() => (isDeleting = true)}>
				<Icon name="delete" />
			</button>
		</p>
	{/if}
</div>

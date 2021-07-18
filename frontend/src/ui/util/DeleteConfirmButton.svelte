<script lang="ts">
	import { createEventDispatcher } from "svelte";
	import Icon from "../icon/Icon.svelte";

	export let disabled = false;
	export let isConfirming = false;

	const dispatch = createEventDispatcher<{
		delete: undefined;
	}>();

	function deleteConfirmed() {
		isConfirming = false;
		dispatch("delete");
	}
</script>

<div class="field has-addons">
	{#if isConfirming}
		<p class="control">
			<button class="button" on:click={() => (isConfirming = false)}>
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
				on:click={() => (isConfirming = true)}>
				<Icon name="delete" />
			</button>
		</p>
	{/if}
</div>

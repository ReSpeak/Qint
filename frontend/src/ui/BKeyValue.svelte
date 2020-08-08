<script lang="typescript">
	export let label: string;
	export let narrow: boolean = false;
</script>

<svelte:options immutable="{true}" />
<!-- svelte-ignore a11y-label-has-associated-control -->
<div class="field is-horizontal">
	<div class="field-label is-normal">
		<label class="label">{label}</label>
	</div>
	<div class="field-body">
		<div class="field" class:is-narrow="{narrow}">
			<slot />
		</div>
	</div>
</div>

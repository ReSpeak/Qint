<svelte:options immutable />
<script lang="ts">
	import Icon from "./Icon.svelte";

	export let text: string;
</script>

<Icon name="orbit mdi-spin" />
<span>{text}</span>

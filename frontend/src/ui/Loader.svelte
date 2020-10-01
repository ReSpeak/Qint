<svelte:options immutable />
<script lang="typescript">
	import Icon from "./Icon.svelte";

	export let text: string;
</script>

<Icon name="orbit mdi-spin" />
<span>{text}</span>

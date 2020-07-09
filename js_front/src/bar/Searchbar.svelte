<script lang="typescript">
	import { writable } from "svelte/store";

	let searchValue = "";
	export let filter = writable("");

	$: filter.set(searchValue);
</script>

<aside class="searchbar">
	<div class="media-content">
		<p class="control has-icons-right">
			<input class="input" type="text" placeholder="Search" bind:value={searchValue} />
			<span class="icon is-small is-right">
				<i class="mdi mdi-magnify mdi-dark"></i>
			</span>
		</p>
	</div>
</aside>

<style lang="scss">
	.searchbar {
		background-color: $box-background-color;
		border-right: rgb(179, 179, 179) 2px solid;
	}

	.searchbar, .media-content, p, input.input, p.control.has-icons-right > span.icon {
		height: 100%;
	}
</style>

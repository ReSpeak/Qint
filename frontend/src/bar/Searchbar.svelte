<script lang="ts">
	export let filter: string;
	export let visible: boolean = true;
</script>

<aside class="searchbar" class:hidden={!visible}>
	<div class="media-content">
		<p class="control has-icons-right">
			<input class="input" type="text" placeholder="Search" bind:value={filter} />
			<span class="icon is-small is-right">
				<i class="mdi mdi-magnify" />
			</span>
		</p>
	</div>
</aside>

<style lang="scss">
	@use "../index.scss" as *;

	.searchbar {
		background-color: $box-background-color;
	}

	.searchbar,
	.media-content,
	p,
	input.input,
	p.control.has-icons-right > span.icon {
		height: 100%;
	}

	.searchbar input[type="text"] {
		background-color: transparent;
		color: white;
	}

	.searchbar input[type="text"]::placeholder {
		color: white;
	}
</style>

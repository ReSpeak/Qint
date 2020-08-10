<script lang="typescript">
	export let styled: boolean = true;
</script>

<div on:click class:button={styled} class="stickySlot">
	<slot />
</div>
<div class="stickyDummy" />

<style lang="scss">
	.stickySlot {
		z-index: 15;
		position: sticky;
		top: 0;
		align-items: center;
		justify-content: flex-start;
		background-color: $background;
		border-radius: 0;
		box-shadow: 0 0.3em 0.3em #0005;
		border-top: 2px solid #425575;
		font-weight: bold;
	}
</style>

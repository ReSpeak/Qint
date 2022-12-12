<script lang="ts">
	import Icon from "../icon/Icon.svelte";
</script>

<div class="dropBackground">
	<div
		on:dragleave|preventDefault
		on:dragover|preventDefault
		on:drop|preventDefault
		class="fileDropOverlay"
	>
		<div class="fileDropInnerBorder">
			<Icon name="file-upload-outline" size="12em" />
		</div>
	</div>
</div>

<style lang="scss">
	.dropBackground {
		position: absolute;
		left: 0;
		top: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 104, 0, 0.5);
	}

	.fileDropOverlay {
		position: absolute;
		z-index: 1;
		top: 0;
		bottom: 0;
		left: 0;
		right: 0;

		background-color: rgba(50, 50, 50, 0.5);
		padding: 2em;
	}

	.fileDropInnerBorder {
		display: flex;
		justify-content: center;
		align-items: center;

		border: 0.5em rgb(60, 60, 60) dashed;
		width: 100%;
		height: 100%;
		border-radius: 3em;
		pointer-events: none;
	}
</style>

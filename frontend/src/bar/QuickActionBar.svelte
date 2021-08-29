<script lang="ts">
	import Icon from "../ui/icon/Icon.svelte";
	
</script>

<div class="actionBar inlineButtons">
	<div class="inlineButton">
		<Icon name="information-outline" />
	</div>
	<div class="inlineButton">
		<Icon name="folder" />
	</div>
</div>

<style lang="scss">
	@import "../style/global_mixin";

	.actionBar {
		background-color: $box-background-color;
		flex: unset !important; // TODO nicer
		width: 2em;
		z-index: 6;
		box-shadow: 3px 0px 5px black;

		flex-direction: column;
	}
</style>
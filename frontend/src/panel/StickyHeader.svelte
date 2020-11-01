<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import { app } from "../app";
	import { DescriptionMode } from "../transientSettings";

	export let title: string;
</script>

<div class="button stickyLine">
	<span>{title}</span>
	<div style="flex:1;" />
	<div class="toolbuttons">
		<slot />
		<button
			class="toolbutton is-small compact active"
			on:click|stopPropagation={() => app.transientSettings.ui._descriptionMode.set(DescriptionMode.None)}>
			<Icon name="close" />
		</button>
	</div>
</div>

<script lang="ts">
	import { Connection } from "../connection";
	import Icon from "../ui/Icon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import ImageFileBrowser from "./ImageFileBrowser.svelte";

	export let connection: Connection;

	let avatarsOpen = false;
</script>

<StickyList>
	<StickySlot styled={false}>
		<StickyHeader title="Icons" />
	</StickySlot>
	<ImageFileBrowser {connection} path={["0", "icons"]} canShowBig={false} />

	<StickySlot on:click={() => (avatarsOpen = true)}>
		<button class="button iconButton" on:click|stopPropagation={() => (avatarsOpen = !avatarsOpen)}>
			<Icon name="chevron-right{avatarsOpen ? ' mdi-rotate-90' : ''}" />
		</button>
		<span>Avatars</span>
	</StickySlot>
	{#if avatarsOpen}
		<!-- We can only delete avatars with their respective clientuid, so we cannot do that here -->
		<ImageFileBrowser {connection} path={["0"]} canUpload={false} canDelete={false} maxSize="10em" />
	{/if}
</StickyList>

<style lang="scss">
</style>

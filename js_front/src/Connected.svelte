<script>
	import SideBar from "./sidebar/SideBar.svelte";
	import UiChat from "./chat/UiChat.svelte";

	export let connection;
</script>

<div class="connected-container">
	<SideBar {connection}/>
	<UiChat {connection}/>
</div>

<style>
	.connected-container > * {
		display: inline-block;
	}
</style>

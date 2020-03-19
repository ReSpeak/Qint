<script>
	import SideBar from "./sidebar/SideBar.svelte";
	import Chat from "./chat/Chat.svelte";

	export let connection;
</script>

<div class="connected-container">
	<SideBar {connection}/>
	<Chat {connection}/>
</div>

<style>
	.connected-container > * {
		display: inline-block;
	}
</style>

<script lang="ts">
	import Icon from "../ui/icon/Icon.svelte";
	import { app } from "../app";
	import { getCurrentWindow } from "@tauri-apps/api/window";

	let isMaximized = false;
	const appSettings = app.settings.app;

	async function updateWindowState() {
		isMaximized = await getCurrentWindow().isMaximized();
	}

	async function toTray() {
		await getCurrentWindow().setSkipTaskbar(true);
		await getCurrentWindow().hide();
	}

	async function minimize() {
		if (appSettings.minimizeToTray) {
			await toTray();
		} else {
			await getCurrentWindow().minimize();
		}
	}

	async function maximize() {
		await updateWindowState();
		if (isMaximized) {
			await getCurrentWindow().unmaximize();
			isMaximized = false;
		} else {
			await getCurrentWindow().maximize();
			isMaximized = true;
		}
	}

	async function close() {
		if (appSettings.closeToTray) {
			await toTray();
		} else {
			app.close();
			await getCurrentWindow().close();
		}
	}

	updateWindowState();
</script>

<div class="inlineButtons">
	<div class="inlineButton minimize" on:click={minimize}>
		<Icon name="minus" />
	</div>
	<div class="inlineButton maximize" on:click={maximize}>
		<Icon name={isMaximized ? "vector-arrange-above" : "crop-square"} />
	</div>
	<div class="inlineButton close" on:click={close}>
		<Icon name="close" />
	</div>
</div>

<style lang="scss">
	@import "../style/global_mixin";

	.minimize,
	.maximize {
		&:hover {
			background-color: $highlight-weak;
		}
	}
	.close {
		&:hover {
			background-color: red;
		}
	}
</style>

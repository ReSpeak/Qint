<script lang="ts">
	import Icon from "../ui/icon/Icon.svelte";
	import { app } from "../app";
	import { appWindow } from "@tauri-apps/api/window";

	let isMaximized = false;
	const appSettings = app.transientSettings.app;

	async function updateWindowState() {
		isMaximized = await appWindow.isMaximized();
	}

	async function toTray() {
		await appWindow.setSkipTaskbar(true);
		await appWindow.hide();
	}

	async function minimize() {
		if (appSettings.minimizeToTray) {
			await toTray();
		} else {
			await appWindow.minimize();
		}
	}

	async function maximize() {
		await updateWindowState();
		if (isMaximized) {
			await appWindow.unmaximize();
			isMaximized = false;
		} else {
			await appWindow.maximize();
			isMaximized = true;
		}
	}

	async function close() {
		if (appSettings.closeToTray) {
			await toTray();
		} else {
			app.close();
			await appWindow.close();
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

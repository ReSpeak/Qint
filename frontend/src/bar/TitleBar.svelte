<script lang="ts">
	import { appWindow } from "@tauri-apps/api/window";
	import Icon from "../ui/icon/Icon.svelte";
	import { onMount } from "svelte";
	import { app } from "../app";

	let isMaximized = false;
	let appSettings = app.transientSettings.app;

	async function startDragWindow() {
		await appWindow.startDragging();
	}

	async function updateWindowState() {
		console.log("checking");
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
			await appWindow.close();
		}
	}

	onMount(() => {
		updateWindowState();
		appWindow.setDecorations(false);

		return () => {
			appWindow.setDecorations(true);
		};
	});
</script>

<div id="titlebar">
	<div class="drag" on:mousedown={startDragWindow} />
	<div class="titleButtons">
		<div class="titleButton minimize" on:click={minimize}>
			<Icon name="minus" />
		</div>
		<div class="titleButton maximize" on:click={maximize}>
			<Icon name={isMaximized ? "vector-arrange-above" : "crop-square"} />
		</div>
		<div class="titleButton close" on:click={close}>
			<Icon name="close" />
		</div>
	</div>
</div>

<style lang="scss">
	@import "../style/global_mixin";

	#titlebar {
		height: 1.5em;
		display: flex;

		background-color: $box-background-color;
	}

	.titleButtons {
		display: flex;
	}

	.titleButton {
		padding: 0 0.5em;
	}
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

	.drag {
		flex: 1;
	}
</style>

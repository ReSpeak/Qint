<script lang="ts">
	import { backend } from "../../backend/backend";
	import TabSlot from "../../ui/container/TabSlot.svelte";
	import Icon from "../../ui/icon/Icon.svelte";
	import FileIO from "../../ui/util/FileIO.svelte";

	let fileIo: FileIO;
	// Plugin names
	let plugins: string[] = [];
	// Created but not yet saved plugins
	let newPlugins: string[] = [];
	let selectedPlugin: string | undefined;
	// Map plugin name to current content
	let editingPlugins: Record<string, string> = {};
	let isLoading = false;
	let editArea: string | undefined;

	$: selectedPluginName = selectedPlugin;
	$: pluginChanged(selectedPlugin);

	loadPlugins();

	async function loadPlugins() {
		try {
			plugins = await backend.plugin_list();
			for (const p of plugins) newPlugins.remove_item(p);
			if (
				selectedPlugin !== undefined &&
				!plugins.includes(selectedPlugin) &&
				!newPlugins.includes(selectedPlugin)
			)
				selectedPlugin = undefined;
		} catch (ex) {
			// TODO: change to debug and show on ui
			console.log("Failed to load plugins: ", ex);
		}
	}

	async function pluginChanged(sel: string | undefined) {
		if (sel !== undefined) {
			editArea = "";
			if (sel in editingPlugins) {
				isLoading = false;
				editArea = editingPlugins[sel];
			} else if (newPlugins.includes(sel)) {
				isLoading = false;
			} else {
				isLoading = true;
				const text = await backend.plugin_get(sel);
				if (selectedPlugin === sel) {
					isLoading = false;
					editArea = text;
				}
			}
		}
	}

	function editAreaChanged() {
		if (selectedPlugin === undefined || editArea === undefined) return;
		editingPlugins[selectedPlugin] = editArea;
	}

	function clickNewPlugin() {
		let name;
		if (!plugins.includes("new.js") && !newPlugins.includes("new.js")) {
			name = "new.js";
		} else {
			let i = 1;
			while (plugins.includes(`new${i}.js`) || newPlugins.includes(`new${i}.js`)) i++;
			name = `new${i}.js`;
		}
		newPlugins.push(name);
		selectedPlugin = name;
	}

	async function clickImportPlugin(files: CustomEvent<FileList>) {
		try {
			const file0 = files.detail[0];
			const content = await file0.text();
			await updatePlugin(file0.name, content);
		} catch (ex) {
			// TODO: change to debug and show on ui
			console.log("Failed to import: ", ex);
		}
	}

	async function updatePlugin(name: string, content: string) {
		await backend.plugin_save(name, content);
		await loadPlugins();
	}

	async function savePlugin() {
		if (
			selectedPluginName === undefined ||
			selectedPlugin === undefined ||
			editArea === undefined
		)
			return;
		try {
			const sel = selectedPlugin;
			if (selectedPluginName === selectedPlugin) {
				// Name unchanged
				await updatePlugin(selectedPlugin, editArea);
			} else {
				await updatePlugin(selectedPluginName, editArea);
				deletePlugin();
				selectedPlugin = selectedPluginName;
			}
			newPlugins.remove_item(sel);
			newPlugins = newPlugins;
			delete editingPlugins[sel];
			editingPlugins = editingPlugins;
		} catch (ex) {
			// TODO: change to debug and show on ui
			console.log("Failed to save: ", ex);
		}
	}

	async function deletePlugin() {
		if (selectedPlugin === undefined) return;
		try {
			delete editingPlugins[selectedPlugin];
			editingPlugins = editingPlugins;
			if (plugins.includes(selectedPlugin)) {
				await backend.plugin_delete(selectedPlugin);
				await loadPlugins();
			} else {
				newPlugins.remove_item(selectedPlugin);
				newPlugins = newPlugins;
				selectedPlugin = undefined;
			}
		} catch (ex) {
			// TODO: change to debug and show on ui
			console.log("Failed to update: ", ex);
		}
	}

	async function exportPlugin(name: string | undefined) {
		if (!name) return;
		const content = await backend.plugin_get(name);
		const blob = new Blob([content], { type: "application/octet-stream" });
		const url = URL.createObjectURL(blob);
		try {
			const a = document.createElement("a");
			a.download = name;
			a.href = url;
			a.click();
		} finally {
			URL.revokeObjectURL(url);
		}
	}
</script>

<!-- svelte-ignore a11y-missing-attribute -->
<TabSlot title="Plugins">
	<div class="layout">
		<div class="pluginList panel is-primary">
			<p class="panel-heading">Your Plugins</p>

			<a class="panel-block is-active" on:click={() => clickNewPlugin()}>
				<Icon name="plus" />
				New
			</a>

			<a class="panel-block is-active" on:click={() => fileIo.askUpload()}>
				<Icon name="file-upload-outline" />
				Import
			</a>

			<div class="panel-block" style="padding: 0" />

			<div class="items">
				{#each plugins as plugin}
					<a
						class="panel-block"
						class:is-active={selectedPlugin === plugin}
						on:click={() => (selectedPlugin = plugin)}>
						<Icon name="toy-brick-outline" />
						<span class:isSelected={selectedPlugin === plugin}>{plugin}</span>
					</a>
				{/each}
				{#each newPlugins as plugin}
					<a
						class="panel-block"
						class:is-active={selectedPlugin === plugin}
						on:click={() => (selectedPlugin = plugin)}>
						<Icon name="toy-brick-outline" />
						<span class:isSelected={selectedPlugin === plugin}>{plugin}</span>
					</a>
				{/each}
			</div>
		</div>

		<form class="pluginOption" on:submit|preventDefault={savePlugin}>
			{#if selectedPlugin !== undefined}
				<div class="buttons is-right">
					<button type="button" class="button is-danger" on:click={deletePlugin}>
						<Icon name="delete" />
						<span>Delete</span>
					</button>

					<span style="flex:1;" />

					<button class="button is-info" on:click={() => exportPlugin(selectedPlugin)}>
						<Icon name="file-export-outline" />
						<span>Download</span>
					</button>

					<button
						type="submit"
						class="button is-success"
						disabled={!(selectedPlugin in editingPlugins) &&
							selectedPluginName === selectedPlugin}>
						<Icon name="content-save" />
						<span>Save</span>
					</button>
				</div>

				<div class="is-horizontal field">
					<input type="text" bind:value={selectedPluginName} class="input" />
				</div>

				<div class="control" class:is-loading={isLoading}>
					<textarea
						bind:value={editArea}
						class="textarea editArea"
						on:keyup={editAreaChanged}
						on:change={editAreaChanged}
						disabled={isLoading} />
				</div>
			{/if}
		</form>
	</div>

	<FileIO bind:this={fileIo} on:uploadRequest={clickImportPlugin} />
</TabSlot>

<style lang="scss">
	.layout {
		width: 100%;
		height: 100%;
		display: grid;
		grid-template-columns: minmax(max-content, 20em) 1fr;
		grid-template-rows: 1fr;
	}

	.pluginList {
		overflow-y: hidden;
		display: flex;
		flex-direction: column;
		background-color: $box-background-color;
	}

	.items {
		overflow-y: auto;
	}

	.pluginOption {
		margin-left: 2em;
		display: grid;
		grid-template-rows: min-content min-content 1fr;
	}

	.isSelected {
		font-weight: bold;
	}

	textarea.textarea.editArea {
		min-height: 90%;
		max-height: inherit;
	}
</style>

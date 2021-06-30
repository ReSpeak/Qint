<script lang="ts">
	import { BASE_ADDRESS } from "../../util";
	import { backend } from "../../backend/backend";
	import TabSlot from "../../ui/container/TabSlot.svelte";
	import Icon from "../../ui/icon/Icon.svelte";

	let dummyUploader: HTMLInputElement;
	let dummyDownloader: HTMLIFrameElement;
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
			const req = await backend.fetch("/plugins");
			plugins = await req.json();
			for (const p of plugins)
				newPlugins.remove_item(p);
			if (selectedPlugin !== undefined && !plugins.includes(selectedPlugin) && !newPlugins.includes(selectedPlugin))
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
				const req = await backend.fetch(`/plugins/${sel}`);
				const text = await req.text();
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

	async function clickImportPlugin() {
		try {
			const files = dummyUploader.files;
			if (files && files.length > 0) {
				const content = await files[0].text();
				await updatePlugin(files[0].name, content);
				dummyUploader.value = null!;
			}
		} catch (ex) {
			// TODO: change to debug and show on ui
			console.log("Failed to import: ", ex);
		}
	}

	async function updatePlugin(name: string, content: string) {
		const req = await backend.fetch(`/plugins/${name}`, {
			method: "PUT",
			body: content,
		});
		await loadPlugins();
	}

	async function savePlugin() {
		if (selectedPluginName === undefined || selectedPlugin === undefined || editArea === undefined) return;
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
				const req = await backend.fetch(`/plugins/${selectedPlugin}`, {
					method: "DELETE",
				});
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

			<a class="panel-block is-active" on:click={() => dummyUploader.click()}>
				<Icon name="file-upload-outline" />
				Import
			</a>

			<div class="panel-block" style="padding: 0" />

			<div class="items">
				{#each plugins as plugin, index}
					<a
						class="panel-block"
						class:is-active={selectedPlugin === plugin}
						on:click={() => (selectedPlugin = plugin)}>
						<Icon name="toy-brick-outline" />
						<span class:isSelected={selectedPlugin === plugin}>{plugin}</span>
					</a>
				{/each}
				{#each newPlugins as plugin, index}
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
				<p class="buttons is-right">
					<button
						type="button"
						class="button is-danger"
						on:click={deletePlugin}>
						<Icon name="delete" />
						<span>Delete</span>
					</button>

					<span style="flex:1;" />

					<a
						class="button is-info"
						download={selectedPlugin}
						target="_blank"
						href="{BASE_ADDRESS}/plugins/{selectedPlugin}">
						<Icon name="file-export-outline" />
						<span>Download</span>
					</a>

					<button type="submit" class="button is-success" disabled={!(selectedPlugin in editingPlugins) && selectedPluginName === selectedPlugin}>
						<Icon name="content-save" />
						<span>Save</span>
					</button>
				</p>

				<div class="is-horizontal field">
					<input type="text" bind:value={selectedPluginName} class="input" />
				</div>

				<div class="control" class:is-loading={isLoading}>
					<textarea bind:value={editArea} class="textarea editArea" on:keyup={editAreaChanged} on:change={editAreaChanged} disabled={isLoading}></textarea>
				</div>
			{/if}
		</form>
	</div>

	<input
		title="Dummy Uploader"
		style="display: none;"
		bind:this={dummyUploader}
		on:change={clickImportPlugin}
		type="file" />
	<iframe
		title="Dummy Downloader"
		style="display: none;"
		bind:this={dummyDownloader}
		sandbox="allow-downloads" />
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

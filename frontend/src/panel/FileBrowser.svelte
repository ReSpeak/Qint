<script lang="typescript">
	import type { ChannelId } from "../ts";
	import { Connection } from "../connection";
	import Icon from "../ui/Icon.svelte";
	import BTable from "../ui/BTable.svelte";
	import type { IColumns, IRows } from "../ui/table";
	import { FolderState } from "../fileTreeCache";
	import type { FileTreeNode } from "../fileTreeCache";
	import { extensionToIcon, formatBytes, pathJoin } from "./fileUtil";
	import { on } from "../util";

	export let connection: Connection;
	export let channelId: ChannelId;
	$: fileTreeCache = connection.fileTreeCache;

	let path: string[] = [];
	let displayChannel: FileTreeNode | null;
	let displayChildren: FileTreeNode[];
	let dummyDownloader: HTMLIFrameElement;
	let invalidateCache = true;
	let fileSelection: FileTreeNode[] = [];
	let creatingNewFolder = false;
	let createNewFolderName = "";
	let deletingFiles = false;

	$: channelRaw = connection.book.channels.get(channelId)!;
	$: channel = $channelRaw;
	$: on(channelId, channelChanged());
	$: {
		on(path);
		const cachePath = getCachePath();
		displayChannel = $fileTreeCache.get(cachePath, true);
		let childrenIter = displayChannel?.children?.values();
		displayChildren = childrenIter !== undefined ? Array.from(childrenIter) : [];
	}
	$: on(channelId, path, refreshCurrentFolder(true));

	function channelChanged() {
		path = [];
		invalidateCache = true;
	}

	function getCachePath(): string[] {
		return [channelId, ...path];
	}

	function refreshCurrentFolder(useCache: boolean) {
		const cachePath = getCachePath();
		if (useCache && !invalidateCache) {
			const cachedFolder = $fileTreeCache.get(cachePath, true);
			if (cachedFolder !== null && cachedFolder.contentLoaded !== FolderState.Dummy) {
				console.log("cached");
				return;
			}
		}
		invalidateCache = false;
		$fileTreeCache.clear(cachePath);
		let getPath = "/" + path.join("/");
		connection.sendMessage({
			Change: {
				ChannelFileListRequest: {
					id: channelId,
					password: "", // TODO
					path: getPath,
				},
			},
		});
	}

	function goUp(toLevel?: number) {
		if (path.length === 0) return;
		path = path.slice(0, toLevel ?? path.length - 1);
	}

	function pushFolder(name: string) {
		path.push(name);
		path = path;
	}

	function onClick(
		evt: CustomEvent<{ event: MouseEvent; row: FileTreeNode; dblclick: boolean }>
	) {
		let { event, row, dblclick } = evt.detail;
		if (dblclick) {
			if (row.isFile) {
				const cachePathStr = getCachePath().join("/");
				const fileUrl = `${connection.backend.serverFileSrc}/file/${cachePathStr}/${
					row.name
				}?dl=${encodeURIComponent(row.name)}`;
				console.log(fileUrl);
				dummyDownloader.src = fileUrl;
			} else {
				pushFolder(row.name);
			}
		}
	}

	function createNewFolderClick() {
		creatingNewFolder = !creatingNewFolder;
		createNewFolderName = "";
	}

	function createNewFolder() {
		const createPath = pathJoin(...path, createNewFolderName);
		connection.sendMessage({
			Change: {
				ChannelCreateDirectory: {
					id: channelId,
					password: "", // TODO
					path: createPath,
				},
			},
		});
		creatingNewFolder = false;
		refreshCurrentFolder(false); // TODO apply in chage instead
	}

	function deleteFiles() {
		// TODO as one packet
		for (let toDelete of fileSelection) {
			const deletePath = pathJoin(...path, toDelete.name);
			connection.sendMessage({
				Change: {
					ChannelDeleteFile: {
						id: channelId,
						password: "", // TODO
						path: deletePath,
					},
				},
			});
		}
		deletingFiles = false;
		refreshCurrentFolder(false); // TODO apply in chage instead
	}

	function selectionChanged(evt: CustomEvent<{ selected: FileTreeNode[] }>) {
		fileSelection = evt.detail.selected;
		deletingFiles = false;
		creatingNewFolder = false;
	}

	function focusNewFolderDiag(node: Element, args: any): SvelteTransitionConfig {
		(node as HTMLElement).focus();
		return {};
	}

	const columns: IColumns<FileTreeNode> = [
		{
			key: "type",
			title: "",
			value: (v) => v.isFile,
			sortable: false,
			headerClass: "text-left",
			customRender: true,
		},
		{
			key: "name",
			title: "Name",
			value: (v) => v.name,
			sortable: true,
		},
		{
			key: "size",
			title: "Size",
			value: (v) => (v.isFile ? v.size : 0),
			renderValue: (v) => (v.isFile ? formatBytes(v.size) : ""),
			sortable: true,
		},
		{
			key: "lastModified",
			title: "Last Modified",
			value: (v) => v.lastModified,
			renderValue: (v) => v.lastModified.format("lll"),
			sortable: true,
		},
	];
</script>

<div class="padBox">
	<div class="buttons">
		<button class="button" on:click={() => refreshCurrentFolder(false)}>
			<Icon name="reload" />
		</button>
		<button class="button">
			<Icon name="upload" />
		</button>
		<button class="button" on:click={createNewFolderClick}>
			<Icon name="folder-plus" />
		</button>
		<!-- <div style="flex:1;" /> -->
		<div class="field has-addons">
			{#if !deletingFiles}
				<p class="control">
					<button
						disabled={fileSelection.length === 0}
						class="button is-danger is-outlined"
						on:click={() => (deletingFiles = true)}>
						<Icon name="delete" />
					</button>
				</p>
			{:else}
				<p class="control">
					<button class="button" on:click={() => (deletingFiles = false)}>
						<Icon name="close" />
					</button>
				</p>
				<p class="control">
					<button class="button is-danger" on:click={deleteFiles}>
						<Icon name="delete-alert" />
					</button>
				</p>
			{/if}
		</div>
	</div>

	<nav class="breadcrumb" aria-label="path">
		<ul>
			<li>
				<div
					on:click={() => goUp(0)}
					class="crumb home"
					class:crubclickable={path.length > 0}>
					<Icon name="folder-home" />
					<span>{channel.name}</span>
				</div>
			</li>
			{#each path.slice(0, -1) as folder, dep (folder)}
				<li>
					<div on:click={() => goUp(dep + 1)} class="crumb crubclickable">{folder}</div>
				</li>
			{/each}
			{#if path.length > 0}
				<li class="is-active">
					<div class="crumb is-active">{path[path.length - 1]}</div>
				</li>
			{:else}
				<li />
			{/if}
		</ul>
	</nav>

	<BTable
		{columns}
		rows={displayChildren}
		on:clickRow={onClick}
		sortBy="name"
		on:selectionChanged={selectionChanged}>
		{#if creatingNewFolder}
			<tr
				on:focusout={() => {
					//creatingNewFolder = false;
				}}>
				<td style="text-align: center;vertical-align: middle;">
					<Icon name="folder" />
				</td>
				<td colspan="3">
					<div class="flex">
						<input
							in:focusNewFolderDiag|local
							class="input"
							type="text"
							bind:value={createNewFolderName} />
						<button class="button" on:click={createNewFolder}>
							<Icon name="check" />
						</button>
					</div>
				</td>
			</tr>
		{/if}
		<tr slot="headerCell" let:col>
			{#if col.key === 'type'}
				<div on:click={() => goUp()} class="upIcon">
					<Icon name="arrow-up-circle-outline" />
				</div>
			{/if}
		</tr>
		<tr slot="colCell" let:col let:row>
			{#if col.key === 'type'}
				{#if row.isFile}
					<Icon name={extensionToIcon(row.name)} />
				{:else}
					<Icon name="folder" />
				{/if}
			{/if}
		</tr>
		<tr slot="empty">
			<th class="noFiles" colspan="4">No files</th>
		</tr>
	</BTable>

	<iframe
		title="Dummy Downloader"
		style="display: none;"
		bind:this={dummyDownloader}
		sandbox="allow-downloads" />
</div>

<style lang="scss">
	@import "../global_mixin";

	.padBox {
		padding: 1em;
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.upIcon {
		@include linkLike;
	}

	.breadcrumb {
		margin-top: 1em;
		margin-bottom: 0.5em;

		.crumb {
			padding: 0 0.75em;

			&.crubclickable {
				@include linkLike;
				&:hover {
					color: $text;
				}
			}
		}

		.home {
			display: flex;
			font-weight: bold;
		}
	}

	.noFiles {
		text-align: center;
	}
</style>

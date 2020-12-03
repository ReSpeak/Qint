<script lang="typescript">
	import type { ChannelId } from "../ts";
	import { Connection } from "../connection";
	import Icon from "../ui/Icon.svelte";
	import BTable from "../ui/BTable.svelte";
	import type { IColumns, IRowOptions, ClickRowEvent, IDragOptions, TableSortFn } from "../ui/table";
	import { FolderState } from "../fileTreeCache";
	import type { FileTreeNode } from "../fileTreeCache";
	import { extensionToIcon, formatBytes, pathJoin, pathSplit } from "./fileUtil";
	import { assert, focus, on } from "../util";

	export let connection: Connection;
	export let channelId: ChannelId;
	$: fileTreeCache = connection.fileTreeCache;

	const enum WorkState {
		None,
		CreatingNewFolder,
		DraggingFilesForUpload,
		DeletingFiles,
		EditingFile,
	}

	let currentState = WorkState.None;
	let path: string[] = [];
	let fileBrowserHasFocus = false;
	let fileTable: BTable;
	let displayChannel: FileTreeNode | null;
	let displayChildren: FileTreeNode[];
	let dummyDownloader: HTMLIFrameElement;
	let dummyUploader: HTMLInputElement;
	let invalidateCache = true;
	let fileSelection: FileTreeNode[] = [];
	let createNewFolderName = "";

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
				return;
			}
		}
		invalidateCache = false;
		$fileTreeCache.clear(cachePath);
		let getPath = "/" + path.join("/");
		connection.sendMessage({
			Change: {
				change: {
					ChannelFileListRequest: {
						id: channelId,
						password: "", // TODO
						path: getPath,
					},
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

	function onClickRow(evt: ClickRowEvent<FileTreeNode>) {
		let { row, dblclick } = evt.detail;
		if (dblclick) {
			if (row.isFile) {
				const cachePathStr = getCachePath().join("/");
				const fileUrl = `${connection.backend.serverFileSrc}/file/${cachePathStr}/${
					row.name
				}?dl=${encodeURIComponent(row.name)}`;
				dummyDownloader.src = fileUrl;
			} else {
				pushFolder(row.name);
			}
		}
	}

	function toggleState(s: WorkState) {
		if (currentState !== s) currentState = s;
		else currentState = WorkState.None;
	}

	function createNewFolderClick() {
		toggleState(WorkState.CreatingNewFolder);
		createNewFolderName = "";
	}

	function createNewFolder() {
		const createPath = pathJoin(...path, createNewFolderName);
		connection.sendMessage({
			Change: {
				change: {
					ChannelCreateDirectory: {
						id: channelId,
						password: "", // TODO
						path: createPath,
					},
				},
			},
		});
		currentState = WorkState.None;
		refreshCurrentFolder(false); // TODO apply in chage instead
	}

	function deleteFiles() {
		// TODO as one packet
		for (let toDelete of fileSelection) {
			const deletePath = pathJoin(...path, toDelete.name);
			connection.sendMessage({
				Change: {
					change: {
						ChannelDeleteFile: {
							id: channelId,
							password: "", // TODO
							path: deletePath,
						},
					},
				},
			});
		}
		currentState = WorkState.None;
		refreshCurrentFolder(false); // TODO apply in chage instead
	}

	function selectionChanged(evt: CustomEvent<{ selected: FileTreeNode[] }>) {
		fileSelection = evt.detail.selected;
		currentState = WorkState.None;
	}

	/// Sort first by type and then by f
	function sortFoldersFirst(f: TableSortFn<FileTreeNode>): TableSortFn<FileTreeNode> {
		return (a, b, order) => {
			if (a.isFile !== b.isFile)
				return a.isFile ? 1 : -1;
			return f(a, b, order);
		};
	}

	const sortOpt = { sensitivity: "base" };
	const columns: IColumns<FileTreeNode> = [
		{
			key: "type",
			title: "",
			value: (v) => v.isFile,
			headerClass: "text-left",
			customRender: true,
		},
		{
			key: "name",
			title: "Name",
			value: (v) => v.name,
			sort: sortFoldersFirst((a, b, order) => a.name.localeCompare(b.name, undefined, sortOpt) * order),
		},
		{
			key: "size",
			title: "Size",
			value: (v) => (v.isFile ? v.size : 0),
			renderValue: (v) => (v.isFile ? formatBytes(v.size) : ""),
			sort: sortFoldersFirst((a, b, order) => ((a.isFile ? a.size : -1) - (b.isFile ? b.size : -1)) * order),
		},
		{
			key: "lastModified",
			title: "Last Modified",
			value: (v) => v.lastModified,
			renderValue: (v) => v.lastModified.format("D.M.YY HH:mm"),
			sort: sortFoldersFirst((a, b, order) => a.lastModified.isAfter(b.lastModified) ? order : -order),
		},
	];
	const rowOptions: IRowOptions<FileTreeNode> = {
		dataType: (t) => (t.isFile ? null : "folder"),
		dataValue: (t) => (t.isFile ? null : pathJoin(...path, t.name)),
	};
	const dragOptions: IDragOptions = {
		dropFilter: () =>
			Array.from(document.querySelectorAll("[data-type='folder']:not(.selected)")),
	};

	let currentUploadTask: any = undefined; // TODO
	function uploadFiles(...files: File[]) {
		connection.filetransferManager.uploadFiles(
			...files.map((file) => {
				return {
					data: file,
					channelId,
					path: pathJoin(...path, file.name),
				};
			})
		);
	}

	function dragEnter(e: DragEvent) {
		currentState = WorkState.DraggingFilesForUpload;
		e.preventDefault();
	}

	function dragLeave(e: DragEvent) {
		currentState = WorkState.None;
		e.preventDefault();
	}

	function dragOver(e: DragEvent) {
		e.preventDefault();
	}

	function dragDrop(e: DragEvent) {
		currentState = WorkState.None;
		e.preventDefault();

		let files = e.dataTransfer?.files;
		if (!files) return;
		uploadFiles(...files);
	}

	function uploadSelected(e: Event) {
		let files = dummyUploader.files;
		if (files && files.length > 0) {
			uploadFiles(...files);
			dummyUploader.value = null!;
		}
	}

	function clickBackground(this: HTMLElement, e: MouseEvent) {
		if (this !== e.target) return;
		fileTable.clearSelection();
		currentState = WorkState.None;
	}

	function onHotkey(e: KeyboardEvent) {
		if (!fileBrowserHasFocus) return;
		if ((e.target as HTMLElement).tagName === "INPUT") return;

		console.log(e.key);

		if (e.key === "F2" && fileSelection.length === 1) {
			e.preventDefault();
			currentState = WorkState.EditingFile;
		}
		if (e.key === "Delete") {
			e.preventDefault();
			if (currentState !== WorkState.DeletingFiles && fileSelection.length > 0) {
				currentState = WorkState.DeletingFiles;
			} else {
				deleteFiles();
			}
		}
	}

	function clickEditFile() {
		toggleState(WorkState.EditingFile);
	}

	type BTableDDEvent = CustomEvent<{ target: HTMLElement }>;
	function dropTargetEnter(e: BTableDDEvent) {
		const elem = e.detail.target;
		elem.classList.add("dropTarget");
	}
	function dropTargetLeave(e: BTableDDEvent) {
		const elem = e.detail.target;
		elem.classList.remove("dropTarget");
	}
	function dropRowsToTarget(e: BTableDDEvent) {
		const elem = e.detail.target;
		elem.classList.remove("dropTarget");
		assert(elem.dataset.key !== undefined, "type 'folder' node must have a 'key'");
		moveFiles(elem.dataset.key, fileSelection);
	}

	function moveFiles(targetPath: string, files: FileTreeNode[]) {
		for (const file of files) {
			const fromPath = pathJoin(...path, file.name);
			const toPath = pathJoin(targetPath, file.name);
			const fromChannel = channelId;
			const toChannel = undefined;
			const toChannelPassword = toChannel !== undefined ? "" : undefined;

			if (fromChannel === toChannel && fromPath === toPath) continue;

			connection.sendMessage({
				Change: {
					change: {
						ChannelRenameFile: {
							id: fromChannel,
							password: "", // TODO
							fromPath,
							toPath,
							toChannel,
							toChannelPassword, // TODO
						},
					},
				},
			});
		}

		$fileTreeCache.clear(pathSplit(channelId, targetPath));
		invalidateCache = true;
		path = path;
	}
</script>

<div
	on:dragenter={dragEnter}
	on:click={clickBackground}
	on:keydown={onHotkey}
	on:blur={() => (fileBrowserHasFocus = false)}
	on:focus={() => (fileBrowserHasFocus = true)}
	tabindex={0}
	class="padBox">
	{#if currentState === WorkState.DraggingFilesForUpload}
		<div
			on:dragleave={dragLeave}
			on:dragover={dragOver}
			on:drop={dragDrop}
			class="fileDropOverlay">
			<div class="fileDropInnerBorder">
				<Icon name="file-upload-outline" size="12em" />
			</div>
		</div>
	{/if}

	<div class="buttons">
		<button class="button" on:click={() => refreshCurrentFolder(false)}>
			<Icon name="reload" />
		</button>
		<button
			title="Upload files"
			on:click={() => dummyUploader.click()}
			class:is-info={currentUploadTask !== undefined}
			class="button">
			<Icon name={currentUploadTask === undefined ? 'upload' : 'orbit mdi-spin'} />
		</button>
		<button
			class="button"
			class:is-info={currentState === WorkState.CreatingNewFolder}
			on:click={createNewFolderClick}>
			<Icon name="folder-plus" />
		</button>
		<!-- <div style="flex:1;" /> -->
		<button
			class="button"
			disabled={fileSelection.length !== 1}
			class:is-info={currentState === WorkState.EditingFile}
			on:click={clickEditFile}>
			<Icon name="pen" />
		</button>
		<div class="field has-addons">
			{#if currentState === WorkState.DeletingFiles}
				<p class="control">
					<button class="button" on:click={() => (currentState = WorkState.None)}>
						<Icon name="close" />
					</button>
				</p>
				<p class="control">
					<button class="button is-danger" on:click={deleteFiles}>
						<Icon name="delete-alert" />
					</button>
				</p>
			{:else}
				<p class="control">
					<button
						disabled={fileSelection.length === 0}
						class="button is-danger is-outlined"
						on:click={() => (currentState = WorkState.DeletingFiles)}>
						<Icon name="delete" />
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
					data-type="folder"
					data-key="/"
					class="crumb home"
					class:crubclickable={path.length > 0}
					class:selected={path.length === 0}>
					<Icon name="folder-home" />
					<span>{channel.name}</span>
				</div>
			</li>
			{#each path.slice(0, -1) as folder, dep (folder)}
				<li>
					<div
						on:click={() => goUp(dep + 1)}
						data-type="folder"
						data-key={pathJoin(...path.slice(0, dep + 1))}
						class="crumb crubclickable">
						{folder}
					</div>
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
		bind:this={fileTable}
		{columns}
		{rowOptions}
		{dragOptions}
		rows={displayChildren}
		on:clickRow={onClickRow}
		sortBy="name"
		on:selectionChanged={selectionChanged}
		on:dragEnter={dropTargetEnter}
		on:dragLeave={dropTargetLeave}
		on:dragDrop={dropRowsToTarget}>
		{#if currentState === WorkState.CreatingNewFolder}
			<tr>
				<td style="vertical-align: middle;">
					<Icon name="folder" />
				</td>
				<td colspan="3">
					<form
						on:submit|preventDefault={createNewFolder}
						on:keydown={(e) => {
							if (e.key === 'Escape') createNewFolderClick();
						}}
						class="flex">
						<input
							in:focus|local
							class="input mr-2"
							type="text"
							bind:value={createNewFolderName} />
						<button class="button" type="submit">
							<Icon name="check" />
						</button>
					</form>
				</td>
			</tr>
		{/if}
		<tr slot="headerCell" let:col>
			{#if col.key === 'type'}
				<div on:click={() => goUp()} class="upIcon" class:invisible={path.length === 0}>
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

	<input
		title="Dummy Uploader"
		style="display: none;"
		bind:this={dummyUploader}
		on:change={uploadSelected}
		type="file"
		multiple />
	<iframe
		title="Dummy Downloader"
		style="display: none;"
		bind:this={dummyDownloader}
		sandbox="allow-downloads" />
</div>

<style lang="scss">
	@import "../style/global_mixin";

	.padBox {
		padding: 1em;
		display: flex;
		flex-direction: column;
		height: 100%;
		position: relative;

		:global(.dropTarget) {
			background-color: $highlight-strong;
		}
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

	.fileDropOverlay {
		position: absolute;
		z-index: 1;
		top: 0;
		bottom: 0;
		left: 0;
		right: 0;

		background-color: rgba(50, 50, 50, 0.5);
		padding: 2em;
	}

	.fileDropInnerBorder {
		display: flex;
		justify-content: center;
		align-items: center;

		border: 0.5em rgb(60, 60, 60) dashed;
		width: 100%;
		height: 100%;
		border-radius: 3em;
		pointer-events: none;
	}
</style>

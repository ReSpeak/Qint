<script lang="ts">
	import type { ChannelId } from "../ts";
	import { Connection } from "../connection";
	import Icon from "../ui/icon/Icon.svelte";
	import Table from "../ui/html/Table.svelte";
	import StickySlot from "../ui/container/StickySlot.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import type {
		IColumns,
		IRowOptions,
		ClickRowEvent,
		IDragOptions,
		TableSortFn,
	} from "../ui/html/uiTable";
	import { FileTreeCache, FolderState } from "../fileTreeCache";
	import type { FileTreeNode } from "../fileTreeCache";
	import { extensionToIcon, formatBytes, pathJoin, pathSplit } from "./fileUtil";
	import { assert, focus, on } from "../util";
	import type { IFileRequest } from "../backend/backend";
	import DeleteConfirmButton from "../ui/util/DeleteConfirmButton.svelte";
	import type { Readable, Writable } from "svelte/store";
	import { Channel } from "../book";
	import DropOverlay from "../ui/util/DropOverlay.svelte";

	export let connection: Connection;
	export let channelId: ChannelId;

	const enum WorkState {
		None,
		CreatingNewFolder,
		DraggingFilesForUpload,
		EditingFile,
	}

	let fileTreeCache: Writable<FileTreeCache>;
	let channelRaw: Channel | undefined = undefined;
	let channel: Channel | undefined = undefined;
	let currentState = WorkState.None;
	let isConfirmingDelete = false;
	let path: string[] = [];
	let fileBrowserHasFocus = false;
	let fileTable: Table<FileTreeNode>;
	let displayChannel: FileTreeNode | null;
	let displayChildren: FileTreeNode[] = [];
	let invalidateCache = true;
	let fileSelection: FileTreeNode[] = [];
	let createNewFolderName = "";

	$: {
		fileTreeCache = connection.fileTreeCache;
		refreshDisplayList();
	}
	$: {
		on(channelId);
		invalidateCache = true;
		channelRaw = connection.book.channels.get(channelId);
		channel = channelRaw !== undefined ? $channelRaw : undefined;
		setDisplayPath(channel!?.lastFilePath ?? []);
	}
	$: on($fileTreeCache, refreshCurrentFolder(true));

	function getCachePath(): string[] {
		return [channelId, ...path];
	}

	async function setDisplayPath(p: string[]) {
		path = p;
		if (channel !== undefined) channel.lastFilePath = path;
		await refreshCurrentFolder(true);
	}

	async function refreshCurrentFolder(useCache: boolean) {
		const cachePath = getCachePath();

		if (useCache && !invalidateCache) {
			const cachedFolder = $fileTreeCache.get(cachePath, true);
			if (cachedFolder !== null && cachedFolder.contentLoaded !== FolderState.Dummy) {
				refreshDisplayList();
				return;
			}
		}
		invalidateCache = false;
		$fileTreeCache.clear(cachePath);
		const getPath = "/" + path.join("/");
		if (channel !== undefined) {
			const reqPath = {
				id: channelId,
				password: "", // TODO
				path: getPath,
			};
			// - TODO handle empty result error explicitly instead of implicitly
			// (currently the `fileTreeCache.clear` at the to ensures that in case
			// of an empty folder nothing gets shown)
			// - TODO handle error show when missing permission instead of
			// "No files"
			await connection.sendChange({
				ChannelFileListRequest: reqPath,
			});
		} else {
			await connection.sendChange({
				ServerFileListRequest: {
					path: getPath,
				},
			});
		}
		refreshDisplayList();
	}

	function refreshDisplayList() {
		const cachePath = getCachePath();
		displayChannel = $fileTreeCache.get(cachePath, true);
		const childrenIter = displayChannel?.children?.values();
		displayChildren = childrenIter !== undefined ? Array.from(childrenIter) : [];
	}

	function goUp(toLevel?: number) {
		if (path.length === 0) return;
		setDisplayPath(path.slice(0, toLevel ?? path.length - 1));
	}

	function pushFolder(name: string) {
		path.push(name);
		setDisplayPath(path);
	}

	function onClickRow(evt: ClickRowEvent<FileTreeNode>) {
		const { row, dblclick } = evt.detail;
		if (dblclick) {
			if (row.isFile) {
				const filePath = pathJoin(...path, row.name);
				const req: IFileRequest = {
					channel: channelId,
					path: filePath,
					cache: false,
					suggested_name: row.name,
				};
				connection.backend.ask_download(req);
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

	async function createNewFolder() {
		currentState = WorkState.None;
		const createPath = pathJoin(...path, createNewFolderName);
		await connection.sendChange({
			ChannelCreateDirectory: {
				id: channelId,
				password: "", // TODO
				path: createPath,
			},
		});
		refreshCurrentFolder(false);
	}

	async function deleteFiles() {
		currentState = WorkState.None;
		isConfirmingDelete = false;
		// TODO as one packet
		const promises = [];
		for (const toDelete of fileSelection) {
			const deletePath = pathJoin(...path, toDelete.name);
			const promise = connection.sendChange({
				ChannelDeleteFile: {
					id: channelId,
					password: "", // TODO
					path: deletePath,
				},
			});
			promises.push(promise);
		}
		await Promise.allSettled(promises);
		refreshCurrentFolder(false);
	}

	function selectionChanged(evt: CustomEvent<{ selected: FileTreeNode[] }>) {
		fileSelection = evt.detail.selected;
		currentState = WorkState.None;
		isConfirmingDelete = false;
	}

	/// Sort first by type and then by f
	function sortFoldersFirst(f: TableSortFn<FileTreeNode>): TableSortFn<FileTreeNode> {
		return (a, b, order) => {
			if (a.isFile !== b.isFile) return a.isFile ? 1 : -1;
			return f(a, b, order);
		};
	}

	const sortOpt: Intl.CollatorOptions = { sensitivity: "base" };
	const columns: IColumns<FileTreeNode> = [
		{
			key: "type",
			title: "",
			value: (v) => v.isFile,
			headerClass: "text-left colIcon",
			class: "colIcon",
			customRender: true,
		},
		{
			key: "name",
			title: "Name",
			value: (v) => v.name,
			headerClass: "colName",
			class: "colName",
			sort: sortFoldersFirst(
				(a, b, order) => a.name.localeCompare(b.name, undefined, sortOpt) * order
			),
		},
		{
			key: "size",
			title: "Size",
			value: (v) => (v.isFile ? v.size : 0),
			headerClass: "colSize",
			class: "colSize",
			renderValue: (v) => (v.isFile ? formatBytes(v.size) : ""),
			sort: sortFoldersFirst(
				(a, b, order) => ((a.isFile ? a.size : -1) - (b.isFile ? b.size : -1)) * order
			),
		},
		{
			key: "lastModified",
			title: "Last\u00A0Modified",
			value: (v) => v.lastModified,
			headerClass: "colModDate",
			class: "colModDate",
			renderValue: (v) => v.lastModified.format("DD.MM.YY\u00A0HH:mm"),
			sort: sortFoldersFirst((a, b, order) =>
				a.lastModified.isAfter(b.lastModified) ? order : -order
			),
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

	const is_uploading: boolean = false; // TODO

	async function askUpload() {
		try {
			await connection.backend.ask_upload({ Files: [channelId, pathJoin(...path)] });
		} catch {
			return;
		}
		refreshCurrentFolder(false);
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
		isConfirmingDelete = false;
		e.preventDefault();

		const files = e.dataTransfer?.files;
		if (!files) return;
		//uploadFiles(...files); // TAURI TODO FIX
	}

	function clickBackground(this: HTMLElement, e: MouseEvent) {
		if (this !== e.target) return;
		fileTable.clearSelection();
		currentState = WorkState.None;
		isConfirmingDelete = false;
	}

	function onHotkey(e: KeyboardEvent) {
		if (!fileBrowserHasFocus) return;
		if ((e.target as HTMLElement).tagName === "INPUT") return;

		if (e.key === "F2" && fileSelection.length === 1) {
			e.preventDefault();
			currentState = WorkState.EditingFile;
			isConfirmingDelete = false;
		}
		if (e.key === "Delete") {
			e.preventDefault();
			if (!isConfirmingDelete && fileSelection.length > 0) {
				isConfirmingDelete = true;
				currentState = WorkState.None;
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

			connection.sendChange({
				ChannelRenameFile: {
					id: fromChannel,
					password: "", // TODO
					fromPath,
					toPath,
					toChannel,
					toChannelPassword, // TODO
				},
			});
		}

		$fileTreeCache.clear(pathSplit(channelId, targetPath));
		refreshCurrentFolder(false);
	}
</script>

<StickySlot styled={false}>
	<StickyHeader title="Files" />
</StickySlot>
<div
	on:dragenter={dragEnter}
	on:click={clickBackground}
	on:keydown={onHotkey}
	on:blur={() => (fileBrowserHasFocus = false)}
	on:focus={() => (fileBrowserHasFocus = true)}
	tabindex={0}
	class="padBox"
>
	{#if currentState === WorkState.DraggingFilesForUpload}
		<DropOverlay on:dragleave={dragLeave} on:dragover={dragOver} on:drop={dragDrop} />
	{/if}

	<div class="buttons">
		<button class="button" on:click={() => refreshCurrentFolder(false)}>
			<Icon name="reload" />
		</button>
		<button
			title="Upload files"
			on:click={() => askUpload()}
			class:is-info={is_uploading}
			class="button"
		>
			<Icon name={is_uploading ? "orbit mdi-spin" : "upload"} />
		</button>
		<button
			class="button"
			class:is-info={currentState === WorkState.CreatingNewFolder}
			on:click={createNewFolderClick}
		>
			<Icon name="folder-plus" />
		</button>
		<!-- <div style="flex:1;" /> -->
		<button
			class="button"
			disabled={fileSelection.length !== 1}
			class:is-info={currentState === WorkState.EditingFile}
			on:click={clickEditFile}
		>
			<Icon name="pen" />
		</button>
		<DeleteConfirmButton
			disabled={fileSelection.length === 0}
			bind:isConfirming={isConfirmingDelete}
			on:delete={deleteFiles}
		/>
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
					class:selected={path.length === 0}
				>
					<Icon name="folder-home" />
					<span>{channel?.name ?? "Server"}</span>
				</div>
			</li>
			{#each path.slice(0, -1) as folder, dep (folder)}
				<li>
					<div
						on:click={() => goUp(dep + 1)}
						data-type="folder"
						data-key={pathJoin(...path.slice(0, dep + 1))}
						class="crumb crubclickable"
					>
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

	<Table
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
		on:dragDrop={dropRowsToTarget}
	>
		{#if currentState === WorkState.CreatingNewFolder}
			<tr>
				<td style="vertical-align: middle;">
					<Icon name="folder" />
				</td>
				<td colspan="3">
					<form
						on:submit|preventDefault={createNewFolder}
						on:keydown={(e) => {
							if (e.key === "Escape") createNewFolderClick();
						}}
						class="flex"
					>
						<input
							in:focus|local
							class="input mr-2"
							type="text"
							bind:value={createNewFolderName}
						/>
						<button class="button" type="submit">
							<Icon name="check" />
						</button>
					</form>
				</td>
			</tr>
		{/if}
		<tr slot="headerCell" let:col>
			{#if col.key === "type"}
				<div on:click={() => goUp()} class="upIcon" class:invisible={path.length === 0}>
					<Icon name="arrow-up-circle-outline" />
				</div>
			{/if}
		</tr>
		<tr slot="colCell" let:col let:row>
			{#if col.key === "type"}
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
	</Table>
</div>

<style lang="scss">
	@use "../index.scss" as *;
	@import "../style/global_mixin";
	@import "./fileBrowser";

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

	// Table formatting helper

	:global(.colIcon) {
		width: 0.1%;
		padding-right: 0 !important;
	}
	:global(.colName) {
		padding-right: 0 !important;
	}
	:global(.colSize) {
		width: 0.1%;
		padding-right: 0 !important;
	}
	:global(.colModDate) {
		width: 0.1%;
	}
</style>

<script lang="ts">
	import { Connection } from "../connection";
	import Icon from "../ui/icon/Icon.svelte";
	import { FileTreeFolder, FolderState } from "../fileTreeCache";
	import type { FileTreeNode } from "../fileTreeCache";
	import { pathJoin } from "./fileUtil";
	import { base64Encode, on, tsHexDecode } from "../util";
	import ImageModal from "../chat/ImageModal.svelte";
	import DeleteConfirmButton from "../ui/util/DeleteConfirmButton.svelte";

	export let connection: Connection;
	export let path: string[];
	export let canUpload = true;
	export let canDelete = true;
	export let canShowBig = true;
	export let maxSize = "1.5em";
	// Allow only one or zero files to be selected
	export let forSelection = false;
	export let selection: string | undefined = undefined;
	$: fileTreeCache = connection.fileTreeCache;

	const enum WorkState {
		None,
		DraggingFilesForUpload,
	}

	let currentState = WorkState.None;
	let isConfirmingDelete = false;
	let fileBrowserHasFocus = false;
	let displayFiles: FileTreeNode[] = [];
	let fileSelection: FileTreeNode[] = [];
	let invalidateCache = true;
	let showBig: FileTreeNode | undefined = undefined;
	let showBigVisible = false;

	$: updateDisplayFiles($fileTreeCache.get(path, true));
	$: on(connection, path, refreshFiles(true));
	$: on(showBigVisible, hideShowBig());
	$: on(selection, displayFiles, updateSelection());

	function hideShowBig() {
		if (!showBigVisible) showBig = undefined;
	}

	function updateDisplayFiles(folder: FileTreeFolder | null) {
		const childrenIter = folder?.children?.values();
		// Only update if available, we do not want to update icons after fetching avatars
		// (which invalidates the icons/ subfolder).
		if (childrenIter !== undefined) {
			displayFiles = Array.from(childrenIter);
			fileSelection = [];
		}
	}

	// Make sure that the filename in `selection` is selected in the `displayFiles`.
	// This gets called when either `selection` changed or when the file list changed.
	// Set `selection` to undefined if no file with this name can be found.
	function updateSelection() {
		if (!forSelection) return;
		if (selection === undefined) {
			if (fileSelection.length !== 0) {
				fileSelection = [];
			}
		} else {
			const file = displayFiles.find((f) => f.name === selection);
			if (file === undefined) {
				selection = undefined;
			} else {
				fileSelection = [file];
			}
		}
	}

	async function getPath(filename: string): Promise<string> {
		// This only works for icons and avatars.
		// Note especially, that we need to list icons in the icons/ subfolder,
		// however we can only download them from the root folder.
		return (
			(await connection.fileProvider({
				channel: "0",
				path: `/${filename}`,
				cache: true,
			})) ?? ""
		);
	}

	async function refreshFiles(useCache: boolean) {
		if (useCache && !invalidateCache) {
			const cachedFolder = $fileTreeCache.get(path, true);
			if (cachedFolder !== null && cachedFolder.contentLoaded !== FolderState.Dummy) {
				return;
			}
		}
		invalidateCache = false;
		$fileTreeCache.clear(path);
		await connection.sendChange({
			ServerFileListRequest: {
				path: pathJoin(...path.slice(1)),
			},
		});
	}

	async function deleteFiles() {
		const deleteFiles = fileSelection;
		fileSelection = [];
		selection = undefined;
		// TODO as one packet
		for (const toDelete of deleteFiles) {
			let name = toDelete.name;
			if (name.startsWith("avatar_")) {
				// To delete avatars, use the base64 encoding of the avatar
				name = "avatar_" + base64Encode(tsHexDecode(name.substring(7)));
			}
			const deletePath = pathJoin(name);
			await connection.sendChange({
				ServerDeleteFile: {
					path: deletePath,
				},
			});
		}
		currentState = WorkState.None;
		isConfirmingDelete = false;
		refreshFiles(false); // TODO apply in chage instead
	}

	const is_uploading: boolean = false; // TODO

	async function uploadFiles() {
		if (!canUpload) return;
		try {
			await connection.backend.ask_upload("Icon");
		} catch (err) {
			console.log(err);
			return;
		}
		refreshFiles(false); // TODO apply in chage instead
	}

	function dragEnter(e: DragEvent) {
		if (!canUpload) return;
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
		//uploadFiles(...files); // TODO TAURI FIX
	}

	function clickBackground(this: HTMLElement, e: MouseEvent) {
		if (this !== e.target) return;
		fileSelection = [];
		currentState = WorkState.None;
		isConfirmingDelete = false;
	}

	function onFileClick(file: FileTreeNode) {
		if (canDelete || forSelection) {
			if (fileSelection.includes(file)) {
				fileSelection.remove_item(file);
				selection = undefined;
			} else {
				if (forSelection) fileSelection = [];
				fileSelection.push(file);
				selection = file.name;
			}
			// Trigger update
			fileSelection = fileSelection;
		} else if (canShowBig) {
			showBig = file;
			showBigVisible = true;
		}
		currentState = WorkState.None;
		isConfirmingDelete = false;
	}

	function onFileDblClick(file: FileTreeNode) {
		if (canDelete && canShowBig) {
			showBig = file;
			showBigVisible = true;
		}
	}

	function onHotkey(e: KeyboardEvent) {
		if (!fileBrowserHasFocus) return;
		if ((e.target as HTMLElement).tagName === "INPUT") return;

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
</script>

{#if showBig !== undefined}
	{#await getPath(showBig.name) then path}
		<ImageModal src={path} bind:visible={showBigVisible} />
	{/await}
{/if}
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
		<button class="button" on:click={() => refreshFiles(false)}>
			<Icon name="reload" />
		</button>
		{#if canUpload}
			<button
				title="Upload files"
				on:click={() => uploadFiles()}
				class:is-info={is_uploading}
				class="button">
				<Icon name={is_uploading ? "orbit mdi-spin" : "upload"} />
			</button>
		{/if}
		{#if canDelete}
			<DeleteConfirmButton
				disabled={fileSelection.length === 0}
				bind:isConfirming={isConfirmingDelete}
				on:delete={deleteFiles} />
		{/if}
	</div>

	{#if displayFiles.length === 0}
		<div class="noFiles">Empty</div>
	{:else}
		<div class="imageList">
			{#each displayFiles as file (file.name)}
				{#if file.isFile}
					<span
						class="image"
						class:selected={fileSelection.includes(file)}
						on:click={() => onFileClick(file)}
						on:dblclick={() => onFileDblClick(file)}>
						{#await getPath(file.name) then path}
							<img
								src={path}
								alt={file.name}
								title={file.name}
								style="max-width: {maxSize}; max-height: {maxSize};" />
						{/await}
					</span>
				{/if}
			{/each}
		</div>
	{/if}
</div>

<style lang="scss">
	@import "../style/global_mixin";
	@import "./fileBrowser";

	.imageList {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5em;
	}

	.image {
		background-color: $background;
		border-radius: 0.5em;
		padding: 0.5em;
		display: flex;
		vertical-align: middle;
	}

	.image.selected {
		background-color: $highlight-strong;
	}

	.image img {
		object-fit: scale-down;
		/* If the icon is not found and the alt text is displayed */
		overflow: hidden;
	}
</style>

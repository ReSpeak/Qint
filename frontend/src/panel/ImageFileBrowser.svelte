<script lang="ts">
	import { Connection } from "../connection";
	import Icon from "../ui/icon/Icon.svelte";
	import { FileTreeFolder, FolderState } from "../fileTreeCache";
	import type { FileTreeNode } from "../fileTreeCache";
	import { pathJoin } from "./fileUtil";
	import { base64Encode, javaHash, on, tsHexDecode } from "../util";
	import ImageModal from "../chat/ImageModal.svelte";

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
		DeletingFiles,
	}

	type SelectableFileTreeNode = FileTreeNode & { selected?: boolean };

	let currentState = WorkState.None;
	let fileBrowserHasFocus = false;
	let displayFiles: SelectableFileTreeNode[] = [];
	let fileSelection: SelectableFileTreeNode[] = [];
	let dummyDownloader: HTMLIFrameElement;
	let dummyUploader: HTMLInputElement;
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
				for (const f of fileSelection) f.selected = false;
				displayFiles = displayFiles;
				fileSelection = [];
			}
		} else {
			for (const f of fileSelection) {
				if (f.name !== selection) {
					f.selected = false;
					displayFiles = displayFiles;
				}
			}
			if (fileSelection.length !== 1) {
				const file = displayFiles.find((f) => f.name === selection);
				if (file === undefined) {
					selection = undefined;
				} else {
					fileSelection = [file];
					file.selected = true;
					displayFiles = displayFiles;
				}
			}
		}
	}

	function getPath(filename: string): string {
		// This only works for icons and avatars.
		// Note especially, that we need to list icons in the icons/ subfolder,
		// however we can only download them from the root folder.
		return `${connection.backend.serverFileSrc}/file/0/${filename}?cache=true`;
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
		refreshFiles(false); // TODO apply in chage instead
	}

	const currentUploadTask: any = undefined; // TODO
	function uploadFiles(...files: File[]) {
		connection.filetransferManager.uploadFiles(
			...files.map((file) => {
				// Add number to name if filename already exist
				// TODO Has to be an u32
				let number = javaHash(file.name);
				let name = "icon_" + number;
				while (displayFiles.find((node) => node.name === name) !== undefined) {
					number++;
					number &= number; // Truncate to u32
					name = "icon_" + number;
				}
				return {
					data: file,
					channelId: "0",
					path: name,
				};
			})
		);
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
		e.preventDefault();

		const files = e.dataTransfer?.files;
		if (!files) return;
		uploadFiles(...files);
	}

	function uploadSelected() {
		if (!canUpload) return;
		const files = dummyUploader.files;
		if (files && files.length > 0) {
			uploadFiles(...files);
			dummyUploader.value = null!;
		}
	}

	function clickBackground(this: HTMLElement, e: MouseEvent) {
		if (this !== e.target) return;
		for (const f of fileSelection) f.selected = false;
		displayFiles = displayFiles;
		fileSelection = [];
		currentState = WorkState.None;
	}

	function onFileClick(file: SelectableFileTreeNode, i: number) {
		if (canDelete || forSelection) {
			if (fileSelection.includes(file)) {
				fileSelection.remove_item(file);
				displayFiles[i].selected = false;
				selection = undefined;
			} else {
				if (forSelection) {
					for (const f of fileSelection) f.selected = false;
					fileSelection = [];
				}
				fileSelection.push(file);
				selection = file.name;
				displayFiles[i].selected = true;
			}
			fileSelection = fileSelection;
		} else if (canShowBig) {
			showBig = file;
			showBigVisible = true;
		}
		currentState = WorkState.None;
	}

	function onFileDblClick(file: SelectableFileTreeNode) {
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
			if (currentState !== WorkState.DeletingFiles && fileSelection.length > 0) {
				currentState = WorkState.DeletingFiles;
			} else {
				deleteFiles();
			}
		}
	}
</script>

{#if showBig !== undefined}
	<ImageModal src={getPath(showBig.name)} bind:visible={showBigVisible} />
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
				on:click={() => dummyUploader.click()}
				class:is-info={currentUploadTask !== undefined}
				class="button">
				<Icon name={currentUploadTask === undefined ? "upload" : "orbit mdi-spin"} />
			</button>
		{/if}
		{#if canDelete}
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
		{/if}
	</div>

	{#if displayFiles.length === 0}
		<div class="noFiles">Empty</div>
	{:else}
		<div class="imageList">
			{#each displayFiles as file, i (file.name)}
				{#if file.isFile}
					<span
						class="image"
						class:selected={file.selected ?? false}
						on:click={() => onFileClick(file, i)}
						on:dblclick={() => onFileDblClick(file)}>
						<img
							src={getPath(file.name)}
							alt={file.name}
							title={file.name}
							style="max-width: {maxSize}; max-height: {maxSize};" />
					</span>
				{/if}
			{/each}
		</div>
	{/if}

	{#if canUpload}
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
	{/if}
</div>

<style lang="scss">
	@import "../style/global_mixin";

	.padBox {
		padding: 1em;
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		position: relative;

		:global(.dropTarget) {
			background-color: $highlight-strong;
		}
	}

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

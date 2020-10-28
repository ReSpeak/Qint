<script lang="typescript">
	import type { ChannelId } from "../ts";
	import { Connection } from "../connection";
	import Icon from "../ui/Icon.svelte";
	import BTable from "../ui/BTable.svelte";
	import type { IColumns, IRows } from "../ui/table";
	import { FolderState } from "../fileTreeCache";
	import type { FileTreeNode } from "../fileTreeCache";
	import { extensionToIcon, formatBytes } from "./fileUtil";

	export let connection: Connection;
	export let channelId: ChannelId;

	let path: string[] = [];
	const fileTreeCache = connection.fileTreeCache;
	let displayChannel: FileTreeNode | null;
	let displayChildren: FileTreeNode[];
	let dummyDownloader: HTMLIFrameElement;

	$: {
		displayChannel = $fileTreeCache.get(getCachePath(path), true);
		let childrenIter = displayChannel?.children?.values();
		displayChildren = childrenIter !== undefined ? Array.from(childrenIter) : [];
		//console.log("Picked", displayChannel, displayChildren);
	}
	$: channelRaw = connection.book.channels.get(channelId)!;
	$: channel = $channelRaw;

	function getCachePath(p: string[]) {
		return [channelId, ...p];
	}

	function refreshCurrentFolder(useCache: boolean) {
		const cachePath = getCachePath(path);
		if (useCache) {
			const cachedFolder = $fileTreeCache.get(cachePath, true);
			if (cachedFolder !== null && cachedFolder.contentLoaded !== FolderState.Dummy) return;
		}
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
		refreshCurrentFolder(true);
	}

	function onClick(
		evt: CustomEvent<{ event: MouseEvent; row: FileTreeNode; dblclick: boolean }>
	) {
		let { event, row, dblclick } = evt.detail;
		if (dblclick) {
			if (row.isFile) {
				const cachePath = getCachePath(path).join("/");
				const fileUrl = `${connection.backend.serverFileSrc}/file/${cachePath}/${
					row.name
				}?dl=${encodeURIComponent(row.name)}`;
				console.log(fileUrl);
				dummyDownloader.src = fileUrl;
			} else {
				pushFolder(row.name);
			}
		}
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
			renderValue: (v) => v.lastModified.format("lll"), // capitalize
			sortable: true,
		},
	];

	refreshCurrentFolder(false);
</script>

<div class="padBox">
	<div class="buttons">
		<button class="button" on:click={() => refreshCurrentFolder(false)}>
			<Icon name="reload" />
		</button>
		<button class="button">
			<Icon name="upload" />
		</button>
		<button class="button">
			<Icon name="folder-plus" />
		</button>
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
					<div class="crumb is-active" aria-current="page">{path[path.length - 1]}</div>
				</li>
			{:else}
				<li />
			{/if}
		</ul>
	</nav>

	<BTable {columns} rows={displayChildren} on:clickRow={onClick} sortBy="name">
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

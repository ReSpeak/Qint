<script lang="typescript">
	import type { ChannelId } from "../ts";
	import { Connection } from "../connection";
	import Icon from "../ui/Icon.svelte";
	import { FolderState } from "../fileTreeCache";
	import type { FileTreeNode } from "../fileTreeCache";
	import { extensionToIcon, formatBytes } from "./fileUtil";

	export let connection: Connection;
	export let channelId: ChannelId;

	let path: string[] = [];
	let selectedElem: FileTreeNode | null = null;
	const fileTreeCache = connection.fileTreeCache;
	let displayChannel: FileTreeNode | null;
	let displayChildren: FileTreeNode[];
	let dummyDownloader: HTMLIFrameElement;

	$: {
		displayChannel = $fileTreeCache.get(getCachePath(path), true);
		let childrenIter = displayChannel?.children?.values();
		displayChildren = childrenIter !== undefined ? Array.from(childrenIter) : [];
		console.log("Picked", displayChannel, displayChildren);
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

	function clearSelection() {
		selectedElem = null;
	}

	function selectElem(elem: FileTreeNode) {
		selectedElem = elem;
	}

	function onDoubleclick(elem: FileTreeNode) {
		if (elem.isFile) {
			const cachePath = getCachePath(path).join("/");
			const fileUrl = `${connection.backend.serverFileSrc}/file/${cachePath}/${elem.name}?dl=${encodeURIComponent(elem.name)}`;
			console.log(fileUrl);
			dummyDownloader.src = fileUrl;
		} else {
			pushFolder(elem.name);
		}
	}

	refreshCurrentFolder(false);
</script>

<div on:click={clearSelection} class="padBox">
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

	<table class="table">
		<tr>
			<th>
				<div on:click={() => goUp()} class="upIcon">
					<Icon name="arrow-up-circle-outline" />
				</div>
			</th>
			<th>Name</th>
			<th>Size</th>
			<th>Last Modified</th>
		</tr>
		{#each displayChildren as childNode (childNode.name)}
			<tr
				class="elem"
				on:click={() => selectElem(childNode)}
				on:dblclick={() => onDoubleclick(childNode)}>
				{#if childNode.isFile}
					<td>
						<Icon name={extensionToIcon(childNode.name)} />
					</td>
					<td>{childNode.name}</td>
					<td>{formatBytes(childNode.size)}</td>
					<td>{childNode.lastModified.format('lll')}</td>
				{:else}
					<td>
						<Icon name="folder" />
					</td>
					<td colspan="2">{childNode.name}</td>
					<td>{childNode.lastModified.format('lll')}</td>
				{/if}
			</tr>
		{:else}
			<tr>
				<th class="noFiles" colspan="4">No files</th>
			</tr>
		{/each}
	</table>
	<iframe title="Dummy Downloader" style="display: none;" bind:this={dummyDownloader} sandbox="allow-downloads" />
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

	.table {
		width: 100%;

		.elem:hover {
			background-color: $highlight-weak;
			cursor: pointer;
		}
	}

	.noFiles {
		text-align: center;
	}
</style>

<script lang="typescript">
	import TsIcon from "../ui/TsIcon.svelte";
	import ServerGroupIcon from "../ui/ServerGroupIcon.svelte";
	import FilterString from "../ui/FilterString.svelte";
	import Icon from "../ui/Icon.svelte";
	import { Connection } from "../connection";
	import { Client } from "../book";
	import { draggable, DragData } from "../ui/draggable";
	import { findParent, flash, render_updates } from "../util";
	import { afterUpdate } from "svelte";
	import { app, NodeSelection } from "../app";
	import { TalkState } from "../ts";
	import HoverMenu from "./HoverMenu.svelte";

	if (render_updates) afterUpdate(() => flash(div));

	export let connection: Connection;
	export let client: Client;
	export let filter: string;
	export let filterShow: boolean = true;
	let hovered = false;
	let newHover = false;
	let showId = false;
	let thisFilter = "";

	$: isSelected = $client.isSelected;
	$: chat = client.chat;
	$: filterShow = applyFilter(filter, $client);
	let ownClient = client.id === connection.book.ownClientId;
	let div: HTMLElement;

	function setChat() {
		app.select(connection, client);
	}

	function applyFilter(filter: string, client: Client) {
		if (filter === "") {
			if (showId) showId = false;
			if (thisFilter !== filter) thisFilter = filter;
			return true;
		}
		const filterById = filter[0] === "/";
		if (filterById) {
			if (!showId) showId = true;
			if (thisFilter !== filter.substr(1)) thisFilter = filter.substr(1);
			return client.id.toString().includes(filter.substr(1));
		} else {
			if (showId) showId = false;
			if (thisFilter !== filter) thisFilter = filter;
			return client.name.toLowerCase().includes(filter.toLowerCase());
		}
	}

	function hover() {
		hovered = true;
		newHover = true;
	}

	function leave(event: MouseEvent) {
		if (event.relatedTarget) {
			if (div.contains(event.relatedTarget as Node)) {
				return;
			}
		}
		newHover = false;
		setTimeout(() => {
			if (!newHover) hovered = false;
		}, 50);
	}

	function dragStart(ev: CustomEvent<DragData>) {
		ev.detail.dragNode.classList.add("dragStyle");
		const channelTree = findParent(ev.detail.dragNode, ".channel-list")!;
		ev.detail.customData = channelTree;
		// TODO find correct max
		//ev.detail.maxY = channelTree.clientHeight;
		ev.detail.lockX = true;
	}

	function dragDrop(ev: CustomEvent<DragData>) {
		ev.detail.dragNode.classList.remove("dragStyle");
		const hoverOpt: HTMLElement[] = [...ev.detail.customData.querySelectorAll(":hover")];
		const dropTarget = hoverOpt.reverse().find((x) => x.dataset.type === "channel");
		console.log(hoverOpt, dropTarget);
		if (dropTarget !== undefined) {
			console.log("Would drop to", dropTarget.dataset.key);
			connection.moveClient(client.id, dropTarget.dataset.key!);
		}
	}
</script>

<li class="container" class:hidden={!filterShow}>
	<div bind:this={div} on:mouseover={hover} on:mouseout={leave} class="hoverDummy">
		<div
			class:ownClient
			class:isSelected
			class="innerContainer"
			on:click={setChat}
			use:draggable={!!connection}
			on:svddrag={dragStart}
			on:svddrop={dragDrop}
			data-type="client"
			data-key={$client.id}>
			<div class:talking={$client.talking !== TalkState.Off} class="talkWave" />
			<TsIcon type="client" source={$client} {connection} />
			<span class="nameBox" style={$client.color}>
				{#if showId}
					[<FilterString filter={thisFilter} content={$client.id.toString()} />]
				{/if}
				<FilterString filter={showId ? '' : thisFilter} content={$client.name} />
			</span>
			<span class="icons">
				{#if $client.inputMuted}
					<Icon name="microphone-off" style="color: red;" />
				{/if}
				{#if $client.outputMuted}
					<Icon name="volume-off" style="color: red;" />
				{/if}
				{#if $client.awayMessage !== null}
					<Icon name="sleep" style="color: rgb(70,180,255);" />
				{/if}
				{#each $client.serverGroups as grp (grp)}
					<ServerGroupIcon id={grp} {connection} />
				{/each}
				{#if $chat.unreadCount > 0}
					<span class="unreadCount" title={$chat.unreadCount.toString()}>
						{#if $chat.unreadCount >= 100}
							99+
						{:else}
							{$chat.unreadCount}
						{/if}
					</span>
				{/if}
			</span>
		</div>
		{#if hovered}
			<HoverMenu {div} selected={new NodeSelection(connection, client)} />
		{/if}
	</div>
</li>

<style lang="scss">
	@import "./tree";

	.talkWave {
		transition: opacity 0.2s ease-in-out, height 0.2s ease-in-out;
		position: absolute;
		// top: 0;
		right: 0;
		bottom: 0;
		left: 0;

		background-image: url("/talking.svg");
		background-size: 100% auto;
		//-webkit-mask-image: radial-gradient(rgba(0, 0, 0, 1), rgba(0, 0, 0, 0));
		//mask-image: radial-gradient(rgba(0, 0, 0, 1), rgba(0, 0, 0, 0));

		opacity: 0;
		height: 50%;
		z-index: -1;
	}

	.talkWave.talking {
		opacity: 1;
		height: 100%;
	}
</style>

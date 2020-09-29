<script lang="typescript">
	import TsIcon from "../ui/TsIcon.svelte";
	import ClientVolume from "../ui/ClientVolume.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import ServerGroupIcon from "../ui/ServerGroupIcon.svelte";
	import FilterString from "../ui/FilterString.svelte";
	import Icon from "../ui/Icon.svelte";
	import { Connection } from "../connection";
	import { Client, TalkState } from "../book";
	import { draggable, DragData } from "../ui/draggable";
	import { findParent, flash } from "../util";
	import { afterUpdate } from "svelte";

	afterUpdate(() => flash(div));

	export let connection: Connection;
	export let client: Client;
	export let filter: string;
	export let filterShow: boolean = true;
	let hovered = false;
	let newHover = false;
	let showId = false;
	let thisFilter = "";

	$: isSelected = $client.isSelected;
	$: filterShow = applyFilter(filter, $client);
	let ownClient = client.id === connection.book.ownClientId;
	let div: HTMLElement;

	function setChat() {
		connection.chat.selectClient(client);
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
			connection.moveClient(client.id, Number(dropTarget.dataset.key));
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
			data-key={client.id}>
			<div class:talking={$client.talking !== TalkState.Off} class="talkWave" />
			<TsIcon type="client" source={client} {connection} />
			<span class="nameBox" style={client.getColor()}>
				{#if showId}
					[
					<FilterString filter={thisFilter} content={client.id.toString()} />]
				{/if}
				<FilterString filter={showId ? '' : thisFilter} content={$client.name} />
			</span>
			<span class="icons">
				{#if $client.input_muted}
					<Icon name="microphone-off" style="color: red;" />
				{/if}
				{#if $client.output_muted}
					<Icon name="volume-off" style="color: red;" />
				{/if}
				{#if $client.away_message !== null}
					<Icon name="sleep" style="color: rgb(70,180,255);" />
				{/if}
				{#each $client.server_groups as grp (grp)}
					<ServerGroupIcon id={grp} {connection} />
				{/each}
			</span>
		</div>
		{#if hovered}
			<div class="hover menu" style="top: {div.getBoundingClientRect().top}px;">
				<div class="corner" />
				<div class="name">
					<ClientName client={$client} />
					{#if client.away_message !== null && client.away_message.length !== 0}
						({client.away_message})
					{/if}
				</div>
				<ClientVolume client={$client} {connection} />
			</div>
		{/if}
	</div>
</li>

<style lang="scss">
	@import "./tree";

	.hover .name {
		grid-row: 1;
		grid-column: 1 / 3;
	}

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

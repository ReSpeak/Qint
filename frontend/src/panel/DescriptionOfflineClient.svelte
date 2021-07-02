<script lang="ts">
	import Icon from "../ui/Icon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import { on } from "../util";
	import { GraphQlClient } from "../book";
	import UiChangeResult from "../ui/UiChangeResult.svelte";
	import UiEmojiString from "../ui/UiEmojiString.svelte";
	import { app } from "../app";
	import type { ChangePromise } from "../connection";
	import ServerName from "../ui/ServerName.svelte";

	export let client: GraphQlClient;

	const developMode = app.transientSettings.ui._developMode;
	let editing = false;
	let changeRequest: ChangePromise | undefined;
	$: on(client, onClientChanged());

	$: clientsByUid = app.clientsByUid;
	$: curOnline = client.uid === null ? [] : ($clientsByUid.get(client.uidStr) ?? []);

	function onClientChanged() {
		editing = false;
		console.log("Client", client);
	}

	function clickEditMode() {
		editing = true;
	}

	function clickSaveChanges() {
		editing = false;
	}
</script>

<StickyList>
	<StickySlot styled={false}>
		<StickyHeader title="Info">
			{#if editing}
				<button
					class="button is-small is-success"
					on:click|stopPropagation={clickSaveChanges}>
					<Icon name="check" />
					<span>Save</span>
				</button>
				<button
					class="button is-small is-danger"
					on:click|stopPropagation={() => (editing = false)}>
					<Icon name="close" />
					<span>Cancel</span>
				</button>
			{:else}
				<button
					class="button is-small outline-button"
					on:click|stopPropagation={clickEditMode}>
					<Icon name="pencil" />
					<span>Edit</span>
				</button>
			{/if}
		</StickyHeader>
	</StickySlot>
	TODO
	- Known on Servers (offline)
	- Currently online
	- Click to go to chat
	<div class="descGroup" class:editing>
		{#await changeRequest then changeResult}
			{#if changeResult !== undefined}
				<div class="notification is-danger">
					<button
						class="toolbutton is-small"
						style="float: right;"
						on:click={() => (changeRequest = undefined)}>
						<Icon name="close" />
					</button>
					<UiChangeResult result={changeResult} />
				</div>
			{/if}
		{/await}

		<div class="dataLine headLine">
			<ClientName client={$client} />
		</div>

		<div class="descTable">
			{#if $developMode}
				{#if $client.uid !== null}
					<div>Uid:</div>
					<div>{$client.uidStr}</div>
					<div>Uid (emoji):</div>
					<div>
						<UiEmojiString data={$client.uid} />
					</div>
				{/if}
			{/if}
		</div>
		<div class="descTable">
			{#each curOnline as c}
			<div>
				<ClientName connection={c[0]} client={c[1]} /> on <ServerName connection={c[0]} />
			</div>
			{/each}
		</div>
	</div>
	<!-- TODO
	<div class="dataLine">
		<div>Volume:</div>
	</div>
	<ClientVolume {client} {connection} />
	-->
</StickyList>

<style lang="scss">
</style>

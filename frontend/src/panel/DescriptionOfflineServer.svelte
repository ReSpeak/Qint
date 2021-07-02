<script lang="ts">
	import type { ChangePromise } from "../connection";
	import type { GraphQlServer } from "../book";
	import Icon from "../ui/Icon.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import UiChangeResult from "../ui/UiChangeResult.svelte";
	import UiEmojiString from "../ui/UiEmojiString.svelte";
	import { app } from "../app";

	export let server: GraphQlServer;

	const developMode = app.transientSettings.ui._developMode;
	let editing = false;
	let changeRequest: ChangePromise | undefined;

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
			<ServerName {server} />
		</div>
		TODO Show connection history and current connections
		{#if $developMode}
			<div class="dataLine">
				<div>Uid:</div>
				<div>{$server.uidStr}</div>
			</div>
			<div class="dataLine">
				<div>Uid (emoji):</div>
				<div>
					<UiEmojiString data={$server.uid} />
				</div>
			</div>
		{/if}
		Show channel tree
	</div>
</StickyList>

<style lang="scss">
	.dataLine .field {
		width: 100%;
	}

	.dataLine .field .control:first-child {
		width: 100%;
	}
</style>

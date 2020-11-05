<script lang="typescript">
	import { Connection } from "../connection";
	import type { ChangePromise } from "../connection";
	import TsIcon from "../ui/TsIcon.svelte";
	import Icon from "../ui/Icon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import type { ChannelId } from "../ts";
	import { codecToName } from "../book";
	import type { Channel } from "../book";
	import RenderedText from "../ui/RenderedText.svelte";
	import RenderedTextEditor from "../ui/RenderedTextEditor.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import BDropDown from "../ui/BDropDown.svelte";
	import BSlider from "../ui/BSlider.svelte";
	import { ChannelType, Codec } from "../book_events";
	import { on } from "../util";

	export let connection: Connection;
	export let channelId: ChannelId;

	let statsOpen = false;
	let editing = false;

	let channel: Channel;
	$: channelRaw = connection.book.getChannel(channelId)!;
	$: channel = $channelRaw;
	$: clients = channelRaw.clients;
	$: clientCount = $clients.length;
	let formatMaxClients: string | number = 0;
	$: {
		// TODO: calculate inheritance?
		if (channel.maxClients === "Inherited" || channel.maxClients === "Unlimited")
			formatMaxClients = channel.maxClients;
		else formatMaxClients = channel.maxClients?.Limited ?? "unknown";
	}

	$: on(channelId, onChannelChanged());
	function onChannelChanged() {
		editing = false;
	}

	let chanEdit = createPropsCopy();
	$: {
		if (editing) {
			chanEdit = createPropsCopy();
		}
	}

	// Load description
	$: descRequest = connection.sendChange({
		ChannelDescriptionRequest: {
			id: channelId,
		},
	});

	function createPropsCopy() {
		// TODO set password fn
		if (channel === undefined) return {};

		return {
			description: channel.description,
			name: channel.name,
			topic: channel.topic,
			codec: channel.codec,
			codecQuality: channel.codecQuality,
			maxClients: channel.maxClients,
			maxFamilyClients: channel.maxFamilyClients,
			channelType: channel.channelType,
			isUnencrypted: channel.isUnencrypted,
			deleteDelay: channel.deleteDelay,
			neededTalkPower: channel.neededTalkPower,
			phoneticName: channel.phoneticName,
			icon: channel.icon,
		};
	}

	function getPropsDiff() {
		let diff: Record<string, any> = {};
		for (const [key, value] of Object.entries(chanEdit)) {
			if (((channel as any)[key] as any) !== value) {
				diff[key] = value;
			}
		}
		return diff;
	}

	function clickEditMode() {
		editing = true;
		chanEdit = createPropsCopy();
	}

	async function clickSaveChanges() {
		editing = false;

		let diff = getPropsDiff();
		await connection.sendChange({
			ChannelEdit: {
				id: channelId,
				...diff,
			},
		});
	}

	const channelTypeOpt = [
		ChannelType.Permanent,
		ChannelType.SemiPermanent,
		ChannelType.Temporary,
	];
	const codecOpt = [Codec.OpusVoice, Codec.OpusMusic];
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
		<div class="dataLine">
			<TsIcon type="channel" source={channel} {connection} />
			{#if editing}
				<input class="input" type="text" bind:value={chanEdit.name} />
			{:else}
				<span class="headLine">{channel.name}</span>
				<div style="flex: 1;" />
				<div><span class="tag is-primary is-rounded">{channel.channelType}</span></div>
			{/if}
		</div>
		{#if editing}
			<div class="dataLine">
				<div>Type:</div>
				<BDropDown bind:selected={chanEdit.channelType} items={channelTypeOpt} />
			</div>
		{/if}
		<div class="dataLine">
			<span>Topic:</span>
			{#if editing}
				<input class="input" type="text" bind:value={chanEdit.topic} />
			{:else}
				<RenderedText text={channel.topicRendered ?? ''} />
			{/if}
		</div>
		{#if editing}
			<div class="dataLine">
				<div>Codec:</div>
				<BDropDown bind:selected={chanEdit.codec} items={codecOpt} display={codecToName} />
			</div>
			<div class="dataLine">
				<div>Codec Quality:</div>
				<div class="flex1">
					<BSlider
						min={1}
						max={10}
						step={1}
						tooltip={true}
						bind:value={chanEdit.codecQuality} />
				</div>
			</div>
		{:else}
			<div class="dataLine">
				<div>Codec:</div>
				<div>{channel.codec !== null ? codecToName(channel.codec) : 'unknown'}</div>
				<div style="margin: 0 0.3em">@</div>
				<div>{channel.codecQuality}</div>
			</div>
		{/if}
		{#if editing}
			<div class="dataLine">
				<div>Max Clients:</div>
				<div>{chanEdit.maxClients} (TODO)</div>
			</div>
			<div class="dataLine">
				<div>Max Family Clients:</div>
				<div>{chanEdit.maxFamilyClients} (TODO)</div>
			</div>
		{:else}
			<div class="dataLine">
				<div>Current Clients:</div>
				<div>{clientCount} / {formatMaxClients}</div>
			</div>
		{/if}
	</div>
	<hr />
	<div class="description">
		{#if editing}
			<RenderedTextEditor bind:raw={chanEdit.description} />
		{:else}
			{#await descRequest then descRequestResult}
				<!-- Todo check properly -->
				{#if descRequestResult !== undefined}
					<span style="color: red;">Missing permission</span>
				{:else}
					<RenderedText text={channel.descriptionRendered ?? ''} />
				{/if}
			{/await}
		{/if}
	</div>
	<StickySlot>Settings</StickySlot>
	klik here for party
	<StickySlot on:click={() => (statsOpen = true)}>
		<button class="button iconButton" on:click|stopPropagation={() => (statsOpen = !statsOpen)}>
			<Icon name="chevron-right{statsOpen ? ' mdi-rotate-90' : ''}" />
		</button>
		<span>Stats</span>
	</StickySlot>
	{#if statsOpen}Test{/if}
</StickyList>

<style>
	.description {
		margin: 1em;
	}

	.descGroup.editing > *:not(:last-child) {
		margin-bottom: 0.5em;
	}
</style>

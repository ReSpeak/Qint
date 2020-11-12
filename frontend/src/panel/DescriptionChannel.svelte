<script lang="typescript">
	import { Connection } from "../connection";
	import type { ChangePromise } from "../connection";
	import TsIcon from "../ui/TsIcon.svelte";
	import Icon from "../ui/Icon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import { codecToName } from "../book";
	import type { Channel } from "../book";
	import RenderedText from "../ui/RenderedText.svelte";
	import RenderedTextEditor from "../ui/RenderedTextEditor.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import BDropDown from "../ui/BDropDown.svelte";
	import BSlider from "../ui/BSlider.svelte";
	import BDurationPicker from "../ui/BDurationPicker.svelte";
	import { ChannelType, Codec, CodecEncryptionMode } from "../book_events";
	import { durationSerialize, on } from "../util";
	import type { RequiredNN } from "../util";
	import type { Duration } from "moment";

	export let connection: Connection;
	export let channel: Channel;

	let editing = false;

	$: clients = channel.clients;
	$: clientCount = $clients.length;
	$: serverRaw = connection.book.server;
	$: server = $serverRaw;
	let formatMaxClients: string | number = 0;
	$: {
		// TODO: calculate inheritance?
		if (channel.maxClients === "Inherited" || channel.maxClients === "Unlimited")
			formatMaxClients = channel.maxClients;
		else formatMaxClients = channel.maxClients?.Limited ?? "unknown";
	}

	$: on(channel, onChannelChanged());
	function onChannelChanged() {
		editing = false;
	}

	// THIS IS NOT A FULL CHANNEL OBJECT
	type EditProps = RequiredNN<Channel> & {
		_isEncrypted: boolean;
		_deleteDelay: Duration;
		_channelType: ChannelType | "Default";
	};
	let chanEdit: EditProps = createPropsCopy();
	let changeRequest: ChangePromise | undefined;

	$: descRequest = requestDescription($channel);

	function requestDescription(channel: Channel) {
		// Load description if out of date
		if (channel.description === undefined) {
			return connection.sendChange({
				ChannelDescriptionRequest: {
					id: channel.id,
				},
			});
		}
		return undefined;
	}

	function createPropsCopy(): EditProps {
		return {
			description: channel.description,
			name: channel.name,
			topic: channel.topic,
			codec: channel.codec,
			codecQuality: channel.codecQuality,
			maxClients: channel.maxClients,
			maxFamilyClients: channel.maxFamilyClients,
			_channelType: channel.isDefault ? "Default" : channel.channelType,
			_isEncrypted: !channel.isUnencrypted,
			_deleteDelay: channel.deleteDelay,
			neededTalkPower: channel.neededTalkPower,
			phoneticName: channel.phoneticName,
			icon: channel.icon,
		} as any;
	}

	function getPropsDiff() {
		let diff: Record<string, any> = {};
		for (const [key, value] of Object.entries(chanEdit)) {
			if (key.startsWith("_")) continue;
			if (((channel as any)[key] as any) !== value) {
				diff[key] = value;
			}
		}
		if (channel.isUnencrypted !== !chanEdit._isEncrypted)
			diff.isUnencrypted = !chanEdit._isEncrypted;
		if (channel.deleteDelay?.toISOString() !== chanEdit._deleteDelay.toISOString())
			diff.deleteDelay = durationSerialize(chanEdit._deleteDelay);
		if (!channel.isDefault && chanEdit._channelType === "Default") {
			diff.isDefault = true;
			diff.channelType = ChannelType.Permanent;
		}
		if (channel.channelType !== chanEdit._channelType && chanEdit._channelType !== "Default")
			diff.channelType = chanEdit._channelType;
		return diff;
	}

	function clickEditMode() {
		editing = true;
		chanEdit = createPropsCopy();
	}

	function clickSaveChanges() {
		editing = false;

		let diff = getPropsDiff();
		if (Object.keys(diff).length === 0) return;
		changeRequest = connection.sendChange({
			ChannelEdit: {
				id: channel.id,
				...diff,
			},
		});
	}

	const channelTypeOpt = [
		ChannelType.Permanent,
		ChannelType.SemiPermanent,
		ChannelType.Temporary,
		"Default",
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
		{#await changeRequest then changeResult}
			<!-- Todo check properly -->
			{#if changeResult !== undefined}
				<div class="notification is-danger">
					<button
						class="toolbutton is-small"
						style="float: right;"
						on:click={() => (changeRequest = undefined)}>
						<Icon name="close" />
					</button>
					{JSON.stringify(changeResult)}
				</div>
			{/if}
		{/await}

		<div class="dataLine">
			<TsIcon type="channel" source={$channel} {connection} />
			{#if editing}
				<input class="input" type="text" bind:value={chanEdit.name} />
			{:else}
				<span class="headLine">{$channel.name}</span>

				<div style="flex: 1;" />
				<span class="tag is-primary is-rounded">
					{#if $channel.isDefault}
						<Icon name="home" title="Default channel" />
					{:else}{$channel.channelType}{/if}
				</span>
			{/if}
		</div>
		{#if editing}
			<div class="dataLine">
				<div>Type:</div>
				{#if $channel.isDefault}
					<div title="Mark another channel as default to change this type">Default <i>(Permanent)</i></div>
				{:else}
					<BDropDown bind:selected={chanEdit._channelType} items={channelTypeOpt} />
				{/if}
			</div>
			{#if chanEdit._channelType === ChannelType.Temporary}
				<div class="dataLine">
					<div>Delete delay:</div>
					<BDurationPicker bind:duration={chanEdit._deleteDelay} />
				</div>
			{/if}
		{/if}
		<div class="dataLine">
			<span>Topic:</span>
			{#if editing}
				<input class="input" type="text" bind:value={chanEdit.topic} />
			{:else}
				<RenderedText text={$channel.topicRendered ?? ''} />
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
			<div
				class="dataLine"
				class:disabled={server.codecEncryptionMode !== CodecEncryptionMode.PerChannel}>
				<label for="channel_codec_encrypted">Voice encrypted:</label>
				{#if server.codecEncryptionMode === CodecEncryptionMode.ForcedOff}
					<i>(Serverwide disabled)</i>
				{:else if server.codecEncryptionMode === CodecEncryptionMode.ForcedOn}
					<i>(Serverwide enabled)</i>
				{:else}
					<input
						id="channel_codec_encrypted"
						type="checkbox"
						class="checkbox-switch is-info"
						bind:checked={chanEdit._isEncrypted} />
				{/if}
			</div>
		{:else}
			<div class="dataLine">
				<div>Codec:</div>
				<div>{$channel.codec !== null ? codecToName($channel.codec) : 'unknown'}</div>
				<div>&nbsp;@&nbsp;</div>
				<div>{$channel.codecQuality}</div>
				<div>&nbsp;</div>
				{#if server.codecEncryptionMode === CodecEncryptionMode.ForcedOn}
					<Icon name="lock-outline" title="Voice is encrypted (forced by server)" />
				{:else if !channel.isUnencrypted}
					<Icon name="lock-outline" title="Voice is encrypted" />
				{/if}
			</div>
		{/if}
		{#if $channel.neededTalkPower !== 0 || editing}
			<div class="dataLine">
				<span>Required Talk Power:</span>
				{#if editing}
					<input class="input" type="number" bind:value={chanEdit.neededTalkPower} />
				{:else}
					<div>{$channel.neededTalkPower}</div>
				{/if}
			</div>
		{/if}
		{#if editing}
			<div class="dataLine">
				<div>Phonetic name:</div>
				<input class="input" bind:value={chanEdit.phoneticName} />
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
					<RenderedText text={$channel.descriptionRendered ?? ''} />
				{/if}
			{/await}
		{/if}
	</div>
</StickyList>

<style lang="scss">
	.description {
		margin: 1em;
	}

	.descGroup.editing > *:not(:last-child) {
		margin-bottom: 0.5em;
	}

	.disabled {
		color: darken($text, 25);
	}
</style>

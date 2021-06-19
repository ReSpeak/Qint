<script lang="ts">
	import { Connection } from "../connection";
	import type { ChangePromise } from "../connection";
	import TsIcon from "../ui/TsIcon.svelte";
	import Icon from "../ui/Icon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import { codecToName } from "../book";
	import type { Channel } from "../book";
	import { MaxClientsMode } from "../ts";
	import RenderedText from "../ui/RenderedText.svelte";
	import RenderedTextEditor from "../ui/RenderedTextEditor.svelte";
	import BDropDown from "../ui/BDropDown.svelte";
	import BSlider from "../ui/BSlider.svelte";
	import BDurationPicker from "../ui/BDurationPicker.svelte";
	import { ChannelType, Codec, CodecEncryptionMode } from "../book_events";
	import { CLEAR_ICON, durationSerialize, enumValues, iconPathToId, on, PASSWORD_PLACEHOLDER } from "../util";
	import type { RequiredNN, Writeable } from "../util";
	import type { Duration } from "moment";
	import UiChangeResult from "../ui/UiChangeResult.svelte";
	import ImageFileBrowser from "./ImageFileBrowser.svelte";

	export let connection: Connection;
	export let channel: Channel;

	let editing = false;
	let editIcon = false;

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
	type EditProps = Omit<
		Writeable<RequiredNN<Channel>>,
		"description" | "isUnencrypted" | "deleteDelay" | "channelType" | "isDefault"
	> & {
		_description: string;
		_isEncrypted: boolean;
		_deleteDelay: Duration;
		_channelType: ChannelType | "Default";
		_password: string;
	};
	let chanEdit: EditProps = createPropsCopy();
	let chanEditMaxClientsMode: MaxClientsMode;
	let chanEditMaxClientsLimit: number;
	let chanEditMaxFamilyClientsMode: MaxClientsMode;
	let chanEditMaxFamilyClientsLimit: number;
	let changeRequest: ChangePromise | undefined;
	let iconSelection: string | undefined = undefined;

	$: descRequest = requestDescription($channel);

	function requestDescription(channel: Channel) {
		// Load description if out of date
		if (channel.optionalData === null) {
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
			name: channel.name,
			topic: channel.topic,
			codec: channel.codec,
			codecQuality: channel.codecQuality,
			maxClients: channel.maxClients,
			maxFamilyClients: channel.maxFamilyClients,
			_description: channel.optionalData?.description ?? "",
			_channelType: channel.isDefault ? "Default" : channel.channelType,
			_isEncrypted: !channel.isUnencrypted,
			_deleteDelay: channel.deleteDelay,
			_password: "",
			neededTalkPower: channel.neededTalkPower,
			phoneticName: channel.phoneticName,
			icon: channel.icon,
		} as any;
	}

	function getPropsDiff() {
		const diff: Record<string, any> = {};

		if (chanEditMaxClientsMode === MaxClientsMode.Inherited) chanEdit.maxClients = "Inherited";
		else if (chanEditMaxClientsMode === MaxClientsMode.Unlimited)
			chanEdit.maxClients = "Unlimited";
		else if (chanEditMaxClientsMode === MaxClientsMode.Limited)
			chanEdit.maxClients = { Limited: chanEditMaxClientsLimit };

		if (chanEditMaxFamilyClientsMode === MaxClientsMode.Inherited)
			chanEdit.maxFamilyClients = "Inherited";
		else if (chanEditMaxFamilyClientsMode === MaxClientsMode.Unlimited)
			chanEdit.maxFamilyClients = "Unlimited";
		else if (chanEditMaxFamilyClientsMode === MaxClientsMode.Limited)
			chanEdit.maxFamilyClients = { Limited: chanEditMaxFamilyClientsLimit };

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
		if (chanEdit._description !== channel.optionalData?.description)
			diff.description = chanEdit._description;
		if (chanEdit._password !== "") {
			diff.password = chanEdit._password;
			delete diff.hasPassword;
		} else if (diff.hasPassword !== undefined) {
			// Ignore when password is empty
			if (diff.hasPassword) delete diff.hasPassword;
			else diff.password = null;
		}
		return diff;
	}

	function editClearPasword() {
		chanEdit.hasPassword = false;
		chanEdit._password = "";
	}

	function clickEditMode() {
		editing = true;
		chanEdit = createPropsCopy();
		iconSelection = "icon_" + chanEdit.icon;

		if (chanEdit.maxClients === "Inherited") chanEditMaxClientsMode = MaxClientsMode.Inherited;
		else if (chanEdit.maxClients === "Unlimited")
			chanEditMaxClientsMode = MaxClientsMode.Unlimited;
		else {
			chanEditMaxClientsMode = MaxClientsMode.Limited;
			chanEditMaxClientsLimit = chanEdit.maxClients.Limited;
		}

		if (chanEdit.maxFamilyClients === "Inherited")
			chanEditMaxFamilyClientsMode = MaxClientsMode.Inherited;
		else if (chanEdit.maxFamilyClients === "Unlimited")
			chanEditMaxFamilyClientsMode = MaxClientsMode.Unlimited;
		else {
			chanEditMaxFamilyClientsMode = MaxClientsMode.Limited;
			chanEditMaxFamilyClientsLimit = chanEdit.maxFamilyClients.Limited;
		}
	}

	function clickSaveChanges() {
		editing = false;

		const diff = getPropsDiff();
		if (Object.keys(diff).length !== 0) {
			changeRequest = connection.sendChange({
				ChannelEdit: {
					id: channel.id,
					...diff,
				},
			});
		}

		const newIcon = iconPathToId(iconSelection);
		if (newIcon !== channel.icon) {
			changeRequest = connection.sendChange({
				ChannelAddPerm: {
					id: channel.id,
					permissionName: "i_icon_id",
					value: parseInt(newIcon) >> 0, // Cast to signed i32, icon ids are u32s but permission values are i32s
				},
			});
		}
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

		<div class="dataLine">
			{#if editing}
				<button class="button" on:click={() => (editIcon = !editIcon)}>
					<TsIcon type="channel" source={{ icon: iconPathToId(iconSelection) }} {connection} />
				</button>
			{:else}
				<TsIcon type="channel" source={$channel} {connection} />
			{/if}
			{#if editing}
				<input class="input" type="text" bind:value={chanEdit.name} />
			{:else}
				<span class="headLine">{$channel.name}</span>

				<div style="flex: 1;" />
				<span class="tag is-rounded">
					{#if $channel.isDefault}
						<Icon name="home" title="Default channel" />
					{:else}{$channel.channelType}{/if}
				</span>
			{/if}
		</div>
		{#if editing && editIcon}
			<div class="dataLine">
				<ImageFileBrowser {connection} path={["0", "icons"]} canShowBig={false} forSelection={true} bind:selection={iconSelection} />
			</div>
		{/if}

		{#if editing}
			<div class="dataLine">
				<div>Type:</div>
				{#if $channel.isDefault}
					<div title="Mark another channel as default to change this type">
						Default
						<i>(Permanent)</i>
					</div>
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
			<label for="edit_topic">Topic:</label>
			{#if editing}
				<input id="edit_topic" class="input" type="text" bind:value={chanEdit.topic} />
			{:else}{$channel.topic ?? ""}{/if}
		</div>
		{#if editing}
			<div class="dataLine">
				<label for="edit_codec">Codec:</label>
				<BDropDown
					id="edit_codec"
					bind:selected={chanEdit.codec}
					items={codecOpt}
					display={codecToName} />
			</div>
			<div class="dataLine">
				<label for="edit_codecQuality">Codec quality:</label>
				<div class="flex1">
					<BSlider
						id="edit_codecQuality"
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
				<div>{$channel.codec !== null ? codecToName($channel.codec) : "unknown"}</div>
				<div>@</div>
				<div>{$channel.codecQuality}</div>
				<div />
				{#if server.codecEncryptionMode === CodecEncryptionMode.ForcedOn}
					<Icon name="lock-outline" title="Voice is encrypted (forced by server)" />
				{:else if !channel.isUnencrypted}
					<Icon name="lock-outline" title="Voice is encrypted" />
				{/if}
			</div>
		{/if}
		{#if $channel.neededTalkPower !== 0 || editing}
			<div class="dataLine">
				<label for="edit_neededTalkPower">Required talk power:</label>
				<span />
				{#if editing}
					<input
						id="edit_neededTalkPower"
						class="input"
						type="number"
						bind:value={chanEdit.neededTalkPower} />
				{:else}
					<div>{$channel.neededTalkPower}</div>
				{/if}
			</div>
		{/if}
		{#if editing}
			<div class="dataLine">
				<label for="edit_phoneticName">Phonetic name:</label>
				<input id="edit_phoneticName" class="input" bind:value={chanEdit.phoneticName} />
			</div>
		{/if}
		{#if editing}
			<div class="dataLine">
				<label for="edit_maxClients" title="Maximum amount of clients in this channel"
					>Max clients:</label>
				<BDropDown
					id="edit_maxClients"
					bind:selected={chanEditMaxClientsMode}
					items={enumValues(MaxClientsMode)} />
				{#if chanEditMaxClientsMode === MaxClientsMode.Limited}
					<input
						class="input maxClientsLimit"
						type="number"
						bind:value={chanEditMaxClientsLimit} />
				{/if}
			</div>
			<div class="dataLine">
				<label
					for="edit_maxFamilyClients"
					title="Maximum amount of clients in this channel and all subchannels combined"
					>Max family clients:</label>
				<BDropDown
					id="edit_maxFamilyClients"
					bind:selected={chanEditMaxFamilyClientsMode}
					items={enumValues(MaxClientsMode)} />
				{#if chanEditMaxFamilyClientsMode === MaxClientsMode.Limited}
					<input
						class="input maxClientsLimit"
						type="number"
						bind:value={chanEditMaxFamilyClientsLimit} />
				{/if}
			</div>
		{:else}
			<div class="dataLine">
				<div>Current clients:</div>
				<div>
					{clientCount} / {formatMaxClients}
					{#if $channel.maxFamilyClients !== null && $channel.maxFamilyClients !== "Unlimited"}
						{#if $channel.maxFamilyClients === "Inherited"}
							(max clients in family are inherited)
						{:else}
							(max clients in family: {$channel.maxFamilyClients.Limited})
						{/if}
					{/if}
				</div>
			</div>
		{/if}
		{#if editing}
			<div class="dataLine">
				<label for="edit_password">Password:</label>
				<div class="field has-addons">
					<div class="control">
						<input
							id="edit_password"
							class="input"
							type="password"
							bind:value={chanEdit._password}
							placeholder={channel.hasPassword && chanEdit.hasPassword !== false
								? PASSWORD_PLACEHOLDER
								: ""} />
					</div>
					{#if channel.hasPassword && chanEdit.hasPassword !== false}
						<div class="control">
							<button class="button" on:click={editClearPasword}
								><Icon name={CLEAR_ICON} /></button>
						</div>
					{/if}
				</div>
			</div>
		{/if}
	</div>
	<hr />
	<div class="description">
		{#if editing}
			<RenderedTextEditor {connection} bind:raw={chanEdit._description} />
		{:else}
			{#await descRequest then descRequestResult}
				<!-- Todo check properly -->
				{#if descRequestResult !== undefined}
					<span style="color: red;">Missing permission</span>
				{:else}
					<RenderedText
						{connection}
						text={$channel.optionalData?.descriptionRendered ?? ""} />
				{/if}
			{/await}
		{/if}
	</div>
</StickyList>

<style lang="scss">
	.description {
		margin: 1em;

		:global(.editbox) {
			height: 100%;
		}
	}

	.disabled {
		color: darken($text, 25);
	}

	.maxClientsLimit {
		margin-left: 1.5em;
	}
</style>

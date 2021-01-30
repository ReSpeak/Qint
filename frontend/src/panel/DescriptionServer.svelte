<script lang="typescript">
	import { Connection } from "../connection";
	import type { ChangePromise } from "../connection";
	import type { Server } from "../book";
	import moment from "moment";
	import { CLEAR_ICON, enumValues, LONG_DATETIME, PASSWORD_PLACEHOLDER } from "../util";
	import type { RequiredNN, Writeable } from "../util";
	import BDropDown from "../ui/BDropDown.svelte";
	import Icon from "../ui/Icon.svelte";
	import PlatformIcon from "../ui/PlatformIcon.svelte";
	import ServerName from "../ui/ServerName.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import StickyHeader from "./StickyHeader.svelte";
	import RenderedText from "../ui/RenderedText.svelte";
	import RenderedTextEditor from "../ui/RenderedTextEditor.svelte";
	import { CodecEncryptionMode, HostMessageMode, licenseTypeGetDoc, OptionalServerDataGen } from "../book_events";
	import UiChangeResult from "../ui/UiChangeResult.svelte";
	import UiEmojiString from "../ui/UiEmojiString.svelte";
	import { app } from "../app";
	import UiServerLog from "./UiServerLog.svelte";

	export let connection: Connection;
	export let server: Server;

	let developMode = app.transientSettings.ui._developMode;
	let editing = false;
	let logOpen = false;
	$: create_date = $server.created !== undefined ? $server.created : moment.unix(0);

	$: {
		if ($server.optionalData == null) getOptionalData();
	}

	// THIS IS NOT A FULL SERVER OBJECT
	type EditProps = Omit<Writeable<RequiredNN<Server>>,
		""> & {
		_password: string;
	};
	type EditPropsOpt = Writeable<RequiredNN<OptionalServerDataGen>>;
	let [servEdit, servEditOpt] = createPropsCopy();
	let changeRequest: ChangePromise | undefined;

	function createPropsCopy(): [EditProps, EditPropsOpt] {
		let serv: EditProps = {} as any;
		let servOpt: EditPropsOpt = {} as any;
		// *** General
		serv.name = server.name;
		serv.phoneticName = server.phoneticName;
		serv.nickname = server.nickname ?? "";
		serv._password = "";
		serv.maxClients = server.maxClients;
		servOpt.reservedSlots = server.optionalData?.reservedSlots ?? 0;
		serv.icon = server.icon;
		serv.welcomeMessage = server.welcomeMessage;
		// *** Host
		// Host Message
		serv.hostmessage = server.hostmessage;
		serv.hostmessageMode = server.hostmessageMode;
		// Host Banner
		serv.hostbannerGfxUrl = server.hostbannerGfxUrl;
		serv.hostbannerUrl = server.hostbannerUrl;
		serv.hostbannerGfxInterval = server.hostbannerGfxInterval;
		serv.hostbannerMode = server.hostbannerMode;
		// Host Button
		serv.hostbuttonTooltip = server.hostbuttonTooltip;
		serv.hostbuttonUrl = server.hostbuttonUrl;
		// ? Icon URL missing ?
		// *** Integrations (Meh)
		// *** Transfers
		// ? UP/DOWN limits missing x4 ?
		// *** Anti-Flood
		// ? Flood limits missing x3 ?
		// *** Security
		// ? Needed Sec Level missing ?
		serv.codecEncryptionMode = server.codecEncryptionMode;
		// *** Misc
		// Default Groups
		serv.defaultChannelGroup = server.defaultChannelGroup;
		serv.defaultServerGroup = server.defaultServerGroup;
		// ? Channel admin group missing ?
		// Complain
		// ? Complain missing x3 ?
		// Other
		// ? min before silence missing ?
		serv.prioritySpeakerDimmModificator = server.prioritySpeakerDimmModificator;
		serv.tempChannelDefaultDeleteDelay = server.tempChannelDefaultDeleteDelay;
		// ? report to serverlist missing ?
		// *** Logs
		// ? logging missing x6 ?
		return [serv, servOpt];
	}

	function getPropsDiff() {
		let diff: Record<string, any> = {};
		if (servEdit.nickname === "" && server.nickname !== "")
			servEdit.nickname = null!;
		for (const [key, value] of Object.entries(servEdit)) {
			if (key.startsWith("_")) continue;
			if ((server as any)[key] !== value) {
				diff[key] = value;
			}
		}
		if (diff.nickname === null)
			diff.nickname = "";
		if (server.optionalData !== null) {
			let optionalData = server.optionalData as any;
			for (const [key, value] of Object.entries(servEditOpt)) {
				if (key.startsWith("_")) continue;
				if (optionalData[key] !== value) {
					diff[key] = value;
				}
			}
		}
		if (servEdit._password !== "") {
			diff.password = servEdit._password;
			delete diff.hasPassword;
		} else if (diff.hasPassword !== undefined) {
			// Ignore when password is empty
			if (diff.hasPassword)
				delete diff.hasPassword;
			else
				diff.password = null;
		}
		return diff;
	}

	function editClearPasword() {
		servEditOpt.hasPassword = false;
		servEdit._password = "";
	}

	function clickEditMode() {
		editing = true;
		[servEdit, servEditOpt] = createPropsCopy();
	}

	function clickSaveChanges() {
		editing = false;

		let diff = getPropsDiff();
		if (Object.keys(diff).length === 0) return;
		changeRequest = connection.sendChange({
			ServerEdit: {
				...diff,
			},
		});
	}

	async function getOptionalData() {
		await connection
			.sendChange({
				ServerVariablesRequest: {},
			})
			.catch((reason) => {
				console.error("ServerVariablesRequest failed: ", reason);
			});
	}

	function disconnect() {
		connection.disconnect();
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
						on:click={() => changeRequest = undefined}>
						<Icon name="close" />
					</button>
					<UiChangeResult result={changeResult} />
				</div>
			{/if}
		{/await}

		<div class="dataLine headLine">
			<TsIcon type="server" source={$server} {connection} />
			{#if editing}
				<input class="input" type="text" bind:value={servEdit.name} />
			{:else}
				<ServerName {connection} />
			{/if}
		</div>
		{#if editing}
			<div class="dataLine">
				<label for="edit_phoneticName">Phonetic name:</label>
				<input
					id="edit_phoneticName"
					class="input"
					bind:value={servEdit.phoneticName}
					placeholder="Same as name by default" />
			</div>
		{/if}
		<div class="dataLine">
			<div>IPs:</div>
			<div>{$server.ips?.join(', ') ?? ''}</div>
			{#if $server.nickname && !editing}
				<span style="margin-left:1em;">(Nickname: </span>
				<code class="nick">{$server.nickname}</code>
				<span>)</span>
			{/if}
		</div>
		{#if editing}
			<div class="dataLine">
				<label for="edit_nickname">Nickname:</label>
				<input id="edit_nickname" class="input" bind:value={servEdit.nickname} />
			</div>
		{/if}
		<div class="dataLine">
			<div>License:</div>
			<div title={licenseTypeGetDoc($server.license)}>{$server.license}</div>
		</div>
		<div class="dataLine">
			<div>Version:</div>
			<PlatformIcon platform={$server.platform} version={$server.version} />
		</div>
		{#if !editing}
			<div class="dataLine">
				<div>Host message:</div>
				<RenderedText {connection} text={$server.hostmessageRendered ?? ''} />
			</div>
		{/if}
		<div class="dataLine large" class:editing>
			<div>Welcome message:</div>
			{#if editing}
				<RenderedTextEditor {connection} bind:raw={servEdit.welcomeMessage} />
			{:else}
				<RenderedText {connection} text={$server.welcomeMessageRendered ?? ''} />
			{/if}
		</div>
		<div class="dataLine">
			<div>Created:</div>
			<div>{create_date.format(LONG_DATETIME)}</div>
		</div>
		{#if editing}
			<div class="dataLine">
				<div>Max clients:</div>
				<input class="input" type="number" bind:value={servEdit.maxClients} />
			</div>
			<div class="dataLine">
				<div>Reserved slots:</div>
				<input class="input" type="number" bind:value={servEditOpt.reservedSlots} />
			</div>
		{:else}
			<div class="dataLine">
				<div>Current clients:</div>
				<div>{'?'} / {$server.maxClients}</div>
				{#if $server.optionalData !== null && $server.optionalData.reservedSlots > 0}
					<div style="margin-left:0.5em;">
						({$server.optionalData.reservedSlots}
						reserved)
					</div>
				{/if}
			</div>
		{/if}
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
	</div>
	{#if editing}
		<StickySlot>Host</StickySlot>
		<div class="descGroup" class:editing>
			<div class="dataLine large" class:editing>
				<div>Host message:</div>
				<RenderedTextEditor {connection} bind:raw={servEdit.hostmessage} />
			</div>

			<div class="dataLine">
				<div>Host message mode:</div>
				<BDropDown
					bind:selected={servEdit.hostmessageMode}
					items={enumValues(HostMessageMode)} />
			</div>
			(TODO) More ...
		</div>

		<StickySlot>Security</StickySlot>
		<div class="descGroup" class:editing>
			<div class="dataLine">
				<div>Audio encryption mode:</div>
				<BDropDown
					bind:selected={servEdit.codecEncryptionMode}
					items={enumValues(CodecEncryptionMode)} />
			</div>

			<div class="dataLine">
				<div>Password:</div>
				<div class="field has-addons">
					<div class="control">
						<input
							id="edit_password"
							class="input"
							type="password"
							bind:value={servEdit._password}
							placeholder={server.optionalData?.hasPassword && servEditOpt.hasPassword !== false ? PASSWORD_PLACEHOLDER : ""}
						/>
					</div>
					{#if server.optionalData?.hasPassword && servEditOpt.hasPassword !== false}
						<div class="control">
							<button class="button" on:click={editClearPasword}><Icon name={CLEAR_ICON} /></button>
						</div>
					{/if}
				</div>
			</div>
		</div>
	{/if}
	<StickySlot on:click={() => (logOpen = true)}>
		<button class="button iconButton" on:click|stopPropagation={() => logOpen = !logOpen}>
			<Icon name="chevron-right{logOpen ? ' mdi-rotate-90' : ''}" />
		</button>
		<span>Log</span>
	</StickySlot>
	{#if logOpen}
		<div class="descGroup serverLog">
			<UiServerLog {connection} />
		</div>
	{/if}
	<StickySlot>Actions</StickySlot>
	<div class="descGroup">
		<p class="buttons">
			<button class="button is-small is-warning" on:click={disconnect}>
				<Icon name="" />
				<span>Disconnect</span>
			</button>
		</p>
	</div>
</StickyList>

<style lang="scss">
	.dataLine .field {
		width: 100%;
	}

	.dataLine .field .control:first-child {
		width: 100%;
	}

	.nick {
		padding: 0 0.3em;
		margin: 0 0.3em;
		border-radius: 5px;
	}

	.serverLog > :global(.serverLog) {
		max-height: calc(100vh - 15em);
	}
</style>

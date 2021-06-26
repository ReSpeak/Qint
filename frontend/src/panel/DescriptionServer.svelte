<script lang="ts">
	import { Connection } from "../connection";
	import type { ChangePromise } from "../connection";
	import type { Server } from "../book";
	import moment from "moment";
	import {
		CLEAR_ICON,
		enumValues,
		formatDuration,
		iconPathToId,
		LONG_DATETIME,
		on,
		PASSWORD_PLACEHOLDER,
	} from "../util";
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
	import {
		CodecEncryptionMode,
		HostBannerMode,
		HostMessageMode,
		licenseTypeGetDoc,
		OptionalServerDataGen,
	} from "../book_events";
	import UiChangeResult from "../ui/UiChangeResult.svelte";
	import UiEmojiString from "../ui/UiEmojiString.svelte";
	import { app } from "../app";
	import UiServerLog from "./UiServerLog.svelte";
	import ImageFileBrowser from "./ImageFileBrowser.svelte";
	import { onMount } from "svelte";

	export let connection: Connection;
	export let server: Server;

	const developMode = app.transientSettings.ui._developMode;
	let editing = false;
	let editIcon = false;
	let logOpen = false;
	$: create_date = $server.created !== undefined ? $server.created : moment.unix(0);

	$: on($server, $server.optionalData === null ? getOptionalData() : undefined);

	// THIS IS NOT A FULL SERVER OBJECT
	type EditProps = Omit<Writeable<RequiredNN<Server>>, ""> & {
		_password: string;
	};
	type EditPropsOpt = Writeable<RequiredNN<OptionalServerDataGen>>;
	let [servEdit, servEditOpt] = createPropsCopy();
	let changeRequest: ChangePromise | undefined;
	let iconSelection: string | undefined = undefined;

	$: servEdit.icon = iconPathToId(iconSelection);

	function createPropsCopy(): [EditProps, EditPropsOpt] {
		const serv: EditProps = {} as any;
		const servOpt: EditPropsOpt = {} as any;
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
		const diff: Record<string, any> = {};
		if (servEdit.nickname === "" && server.nickname !== "") servEdit.nickname = null!;
		for (const [key, value] of Object.entries(servEdit)) {
			if (key.startsWith("_")) continue;
			if ((server as any)[key] !== value) {
				diff[key] = value;
			}
		}
		if (diff.nickname === null) diff.nickname = "";
		if (server.optionalData !== null) {
			const optionalData = server.optionalData as any;
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
			if (diff.hasPassword) delete diff.hasPassword;
			else diff.password = null;
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
		iconSelection = "icon_" + servEdit.icon;
	}

	function clickSaveChanges() {
		editing = false;

		const diff = getPropsDiff();
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
				console.error("ServerVariablesRequest failed", reason);
			});
	}

	function disconnect() {
		connection.disconnect();
	}

	onMount(() => {
		const timer = setInterval(getOptionalData, 10000);
		return () => clearInterval(timer);
	});
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
			{#if editing}
				<button class="button" on:click={() => (editIcon = !editIcon)}>
					<TsIcon type="server" source={servEdit} {connection} />
				</button>
			{:else}
				<TsIcon type="server" source={$server} {connection} />
			{/if}
			{#if editing}
				<input class="input" type="text" bind:value={servEdit.name} />
			{:else}
				<ServerName {connection} />
				{#if $server.nickname}
					<span style="margin-left:1em;">(Nickname: </span>
					<code class="nick">{$server.nickname}</code>
					<span>)</span>
				{/if}
			{/if}
			<div style="flex: 1;" class="platformIconSpacer" />
			<PlatformIcon platform={$server.platform} version={$server.version} />
		</div>
		{#if editing && editIcon}
			<div class="dataLine">
				<ImageFileBrowser
					{connection}
					path={["0", "icons"]}
					canShowBig={false}
					forSelection={true}
					bind:selection={iconSelection} />
			</div>
		{/if}
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
		{#if editing}
			<div class="dataLine">
				<label for="edit_nickname">Nickname:</label>
				<input id="edit_nickname" class="input" bind:value={servEdit.nickname} />
			</div>
		{/if}
		<div class="dataLine">
			<div>License:</div>
			<div title={$server.license}>{licenseTypeGetDoc($server.license)}</div>
		</div>
		{#if !editing}
			<div class="dataLine">
				<div>Host message:</div>
				<RenderedText {connection} text={$server.hostmessageRendered ?? ""} />
			</div>
		{/if}
		<div class="dataLine large" class:editing>
			<div>Welcome message:</div>
			{#if editing}
				<RenderedTextEditor {connection} bind:raw={servEdit.welcomeMessage} />
			{:else}
				<RenderedText {connection} text={$server.welcomeMessageRendered ?? ""} />
			{/if}
		</div>
		<div class="dataLine">
			<div>Created:</div>
			<div>{create_date.format(LONG_DATETIME)}</div>
		</div>
		{#if $server.optionalData !== null}
			<div class="dataLine">
				<div>Uptime:</div>
				<div>{formatDuration($server.optionalData.uptime)}</div>
			</div>
		{/if}
		{#if editing}
			<div class="dataLine">
				<label for="edit_maxClients">Max clients:</label>
				<input
					id="edit_maxClients"
					class="input"
					type="number"
					bind:value={servEdit.maxClients} />
			</div>
			<div class="dataLine">
				<label for="edit_reservedSlots">Reserved slots:</label>
				<input
					id="edit_reservedSlots"
					class="input"
					type="number"
					bind:value={servEditOpt.reservedSlots} />
			</div>
		{:else}
			<div class="dataLine">
				<div>Current clients:</div>
				<div>{$server.optionalData?.clientCount ?? "?"} / {$server.maxClients}</div>
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
				<div>IPs:</div>
				<div>{$server.ips?.join(", ") ?? ""}</div>
			</div>
			<div class="dataLine">
				<div>Port:</div>
				<div>{$server.optionalData?.port}</div>
			</div>
			<div class="dataLine">
				<div>Id:</div>
				<div>{$server.id}</div>
			</div>
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
		{#if !editing}
			{#if $server.hostbuttonGfxUrl}
				<div class="dataLine">
					<a href={$server.hostbuttonUrl} title={$server.hostbuttonTooltip}>
						<img
							src={$server.hostbuttonGfxUrl}
							alt={$server.hostbuttonTooltip}
							class="hostbutton" />
					</a>
				</div>
			{/if}
			{#if $server.hostbannerGfxUrl}
				<div class="dataLine">
					<a href={$server.hostbannerUrl}>
						<img src={$server.hostbannerGfxUrl} alt="hostbanner" class="hostbanner" />
					</a>
				</div>
			{/if}
		{/if}
	</div>
	{#if editing}
		<StickySlot>Host</StickySlot>
		<div class="descGroup" class:editing>
			<div class="dataLine">
				<h3 class="title">Host message:</h3>
			</div>
			<RenderedTextEditor {connection} bind:raw={servEdit.hostmessage} />

			<div class="dataLine">
				<label for="edit_hostmessageMode">Mode:</label>
				<BDropDown
					id="edit_hostmessageMode"
					bind:selected={servEdit.hostmessageMode}
					items={enumValues(HostMessageMode)} />
			</div>

			<div class="dataLine">
				<h3 class="title">Host banner</h3>
			</div>
			<div class="dataLine">
				<label for="edit_hostbannerUrl">URL:</label>
				<input id="edit_hostbannerUrl" class="input" bind:value={servEdit.hostbannerUrl} />
			</div>
			<div class="dataLine">
				<label for="edit_hostbannerGfxUrl">Image:</label>
				<input
					id="edit_hostbannerGfxUrl"
					class="input"
					bind:value={servEdit.hostbannerGfxUrl} />
			</div>
			<div class="dataLine">
				<label for="edit_hostbannerMode">Mode:</label>
				<BDropDown
					id="edit_hostbannerMode"
					bind:selected={servEdit.hostbannerMode}
					items={enumValues(HostBannerMode)} />
			</div>

			<div class="dataLine">
				<h3 class="title">Host button</h3>
			</div>
			<div class="dataLine">
				<label for="edit_hostbuttonUrl">URL:</label>
				<input id="edit_hostbuttonUrl" class="input" bind:value={servEdit.hostbuttonUrl} />
			</div>
			<div class="dataLine">
				<label for="edit_hostbuttonGfxUrl">Image:</label>
				<input
					id="edit_hostbuttonGfxUrl"
					class="input"
					bind:value={servEdit.hostbuttonGfxUrl} />
			</div>
			<div class="dataLine">
				<label for="edit_hostbuttonTooltip">Tooltip:</label>
				<input
					id="edit_hostbuttonTooltip"
					class="input"
					bind:value={servEdit.hostbuttonTooltip} />
			</div>
		</div>

		<StickySlot>Security</StickySlot>
		<div class="descGroup" class:editing>
			<div class="dataLine">
				<label for="edit_codecEncryptionMode">Audio encryption mode:</label>
				<BDropDown
					id="edit_codecEncryptionMode"
					bind:selected={servEdit.codecEncryptionMode}
					items={enumValues(CodecEncryptionMode)} />
			</div>

			<div class="dataLine">
				<label for="edit_password">Password:</label>
				<div class="field has-addons">
					<div class="control">
						<input
							id="edit_password"
							class="input"
							type="password"
							bind:value={servEdit._password}
							placeholder={server.optionalData?.hasPassword &&
							servEditOpt.hasPassword !== false
								? PASSWORD_PLACEHOLDER
								: ""} />
					</div>
					{#if server.optionalData?.hasPassword && servEditOpt.hasPassword !== false}
						<div class="control">
							<button class="button" on:click={editClearPasword}
								><Icon name={CLEAR_ICON} /></button>
						</div>
					{/if}
				</div>
			</div>
		</div>
	{/if}
	<StickySlot on:click={() => (logOpen = true)}>
		<button class="button iconButton" on:click|stopPropagation={() => (logOpen = !logOpen)}>
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

	.platformIconSpacer {
		margin-right: 0.5em;
	}

	.hostbutton {
		max-width: 5em;
		max-height: 5em;
	}
</style>

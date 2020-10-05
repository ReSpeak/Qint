<script lang="typescript">
	import { get } from "svelte/store";
	import { Connection } from "../connection";
	import type { ServerGroupId } from "../ts";
	//import { Moment } from "moment";
	import Icon from "../ui/Icon.svelte";
	import PlatformIcon from "../ui/PlatformIcon.svelte";
	import ServerGroupIcon from "../ui/ServerGroupIcon.svelte";
	import TsIcon from "../ui/TsIcon.svelte";
	import ClientName from "../ui/ClientName.svelte";
	import StickyList from "../ui/StickyList.svelte";
	import StickySlot from "../ui/StickySlot.svelte";
	import ClientVolume from "../ui/ClientVolume.svelte";
	import { getClientAvatarPath } from "../ui/clientIcon";
	import { Reason } from "../backend/ws";

	export let connection: Connection;
	export let clientId: number;

	const sgs = connection.book.serverGroups;
	$: clientRaw = connection.book.clients.get(clientId)!;
	$: client = $clientRaw;
	$: avatarPath = getClientAvatarPath($client, connection);
	$: ownClient = clientId === connection.book.ownClientId;
	//let onlineSince: Moment; TODO

	interface ExtendedGroup {
		isMember: boolean,
		id: ServerGroupId;
		name: string;
	}

	let groups: ExtendedGroup[];
	$: {
		groups = [];
		$sgs.forEach((group, id) => {
			const g = get(group);
			if (g.group_type === "Regular") {
				groups.push({
					isMember: client.server_groups.includes(id),
					...g
				});
			}
		});
		groups.sort((a, b) => {
			if (a.isMember !== b.isMember)
				return a.isMember ? -1 : 1;
			const nameCmp = a.name.localeCompare(b.name);
			if (nameCmp !== 0)
				return nameCmp;
			return a.id - b.id;
		});
	}

	function changeServerGroup(e: Event, group: ServerGroupId, isMember: boolean) {
		if (e.target instanceof HTMLInputElement)
			e.target.disabled = true;

		if (isMember) {
			connection.sendMessage({
				Change: {
					ClientAddServerGroup: {
						id: clientId,
						server_group: group,
					}
				}
			});
		} else {
			connection.sendMessage({
				Change: {
					ClientRemoveServerGroup: {
						id: clientId,
						server_group: group,
					}
				}
			});
		}
	}

	function kickFromChannel() {
		connection.sendMessage({
			Change: {
				ClientKick: {
					id: clientId,
					reason: Reason.KickChannel,
				}
			}
		});
	}

	function kickFromServer() {
		connection.sendMessage({
			Change: {
				ClientKick: {
					id: clientId,
					reason: Reason.KickServer,
				}
			}
		});
	}
</script>

<StickyList>
	<StickySlot>Info</StickySlot>
	<div class="descGroup">
		<div class="dataLine headLine">
			<TsIcon type="client" source={{icon: $client.icon}} {connection} />
			<ClientName client={$client} />
			<div style="flex: 1;" />
			<div>
				{'Version'}
				<PlatformIcon platform={'Platform'} />
			</div>
		</div>
		<div class="dataLine">
			<div>Description:</div>
			<div>{$client.description}</div>
		</div>
		<div class="dataLine">
			<div>Online since:</div>
			<div>No Data</div>
		</div>
		<div class="dataLine">
			<div>Time away:</div>
			<div>No Data</div>
		</div>
		{#if avatarPath}
			<img class="clientAvatar" src={avatarPath} alt="Client avatar" />
		{/if}
		<div class="serverGroups">
			<div>Server Groups:</div>
			<div class="serverGroupList">
				{#each groups as grp (grp)}
				<label class="checkbox serverGroupContainer" for={"group" + grp.id}>
					<div class="serverGroupCheckbox">
						<input type="checkbox" id={"group" + grp.id} on:input={e => changeServerGroup(e, grp.id, !grp.isMember)} checked={grp.isMember} />
					</div>
					<div class="serverGroupSpacing"></div>
					<div class="serverGroupIcon">
						<ServerGroupIcon id={grp.id} {connection} />
					</div>
					<div class="serverGroupDescription" title={"Id " + grp.id}>{grp.name}</div>
				</label>
				{/each}
			</div>
		</div>
	</div>
	<StickySlot>Actions</StickySlot>
	<div class="descGroup">
		{#if !ownClient}
			<p class="buttons">
				<button class="button is-small is-warning" on:click={kickFromChannel}>
					<Icon name="shoe-formal" />
					<span>Kick Channel</span>
				</button>
				<button class="button is-small is-danger" on:click={kickFromServer}>
					<Icon name="shoe-formal" />
					<span>Kick Server</span>
				</button>
				<button class="button is-small is-danger">
					<Icon name="cancel" />
					<span>Ban</span>
				</button>
			</p>
			<div class="dataLine">
				<div>Volume:</div>
				<ClientVolume {client} {connection} />
			</div>
		{/if}
	</div>
</StickyList>

<style lang="scss">
	.clientAvatar {
		max-width: 100%;
	}

	.serverGroups {
		margin-top: 1.5em;
	}

	.serverGroupList {
		display: table;
		border-spacing: 0 0.1em;
		margin-top: 0.5em;
	}

	.serverGroupContainer {
		display: table-row;
		height: 1.9em;
	}

	.serverGroupContainer > * {
		display: table-cell;
		padding: 0.2em;
		background-color: mix($background, $text, 95%);
		vertical-align: middle;
	}

	.serverGroupCheckbox {
		padding-left: 1em;
		padding-right: 1em;
		margin-right: 0.1em;
		border-radius: 0.2em 0 0 0.2em;

		input {
			vertical-align: middle;
		}
	}

	.serverGroupSpacing {
		width: 0.1em;
		background-color: transparent;
		padding: 0;
	}

	.serverGroupDescription {
		padding-right: 1em;
		border-radius: 0 0.2em 0.2em 0;
	}
</style>

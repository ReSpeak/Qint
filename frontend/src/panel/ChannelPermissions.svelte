<script lang="ts">
	import { Connection } from "../connection";
	import type { Channel } from "../book";
	import { on } from "../util";
	import Icon from "../ui/icon/Icon.svelte";
	import ChangeResult from "../ui/specialized/ChangeResult.svelte";
	import PermissionList from "../ui/specialized/PermissionList.svelte";
	import type { PermissionDiff } from "../ui/specialized/uiPermissionList";
	import type { Permission, PermissionDescription } from "../ts";
	import { ChangePromise } from "../backend/returnCodeTracker";

	export let connection: Connection;
	export let channel: Channel;

	const permFilterList = [
		"i_channel_needed_modify_power",
		"i_channel_needed_delete_power",
		"i_channel_needed_join_power",
		"i_channel_needed_subscribe_power",
		"i_channel_needed_description_view_power",
		"i_client_needed_talk_power",
		"i_ft_needed_file_upload_power",
		"i_ft_needed_file_download_power",
		"i_ft_needed_file_rename_power",
		"i_ft_needed_file_browse_power",
		"i_ft_needed_directory_create_power",
	];

	const permissions = connection.channelPermCache;
	let loadPermissions: ChangePromise | undefined;
	let permissionList: [Permission, PermissionDescription][] = [];
	let uiPermissions: PermissionList;

	$: on(connection, channel, loadPerms());

	export function getDiff(): PermissionDiff {
		return uiPermissions.getDiff();
	}

	async function loadPerms() {
		try {
			$permissions = [];
			loadPermissions = connection.sendChange({
				ChannelPermListRequest: { id: channel.id },
			});
			const permList = await connection.permList.get();
			await loadPermissions;
			permissionList = [];
			for (const [perm, desc] of permList) {
				if (permFilterList.includes(desc.name)) permissionList.push([perm, desc]);
			}
		} catch (e) {
			console.error("Failed to fetch permissions", e);
		}
	}
</script>

{#await loadPermissions then changeResult}
	{#if changeResult !== undefined}
		<div class="notification is-danger">
			<button
				class="toolbutton is-small"
				style="float: right;"
				on:click={() => (loadPermissions = undefined)}>
				<Icon name="close" />
			</button>
			<ChangeResult result={changeResult} />
		</div>
	{/if}
{/await}

<PermissionList bind:this={uiPermissions} {permissionList} permissions={$permissions} />

<style lang="scss">
</style>

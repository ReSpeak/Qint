<script lang="ts">
	import Icon from "../icon/Icon.svelte";
	import type { Permission, PermissionDescription } from "../../ts";
	import { on } from "../../util";
	import { defaultPerm } from "./uiPermissionList";
	import type { InPermissionData, PermissionData, PermissionDiff } from "./uiPermissionList";
	import HighlightString from "./HighlightString.svelte";

	/// Permissions to show
	export let permissionList: [Permission, PermissionDescription][];
	/// Currently set permissions
	export let permissions: InPermissionData[];

	let allPermissions: Record<Permission, PermissionData>;
	// A copy of permissionList, so permissionChanged does not get triggered if a value changes
	let permList: [Permission, PermissionDescription][];

	let filter: string = "";

	$: on(permissionList, permissions, permissionsChanged());

	// Get changes from original permission state
	export function getDiff(): PermissionDiff {
		const added: InPermissionData[] = [];
		const removed: Permission[] = [];
		const allPerms = new Set(permissionList.map(([perm, _desc]) => perm));
		for (const perm of permissions) {
			const newPerm = allPermissions[perm.permissionId];
			if (isDefault(newPerm))
				removed.push(perm.permissionId);
			else if (!isEqual(newPerm, perm))
				added.push({ permissionId: perm.permissionId, ...newPerm });
			allPerms.delete(perm.permissionId);
		}

		for (const perm of allPerms) {
			const newPerm = allPermissions[perm];
			if (!isDefault(newPerm))
				added.push({ permissionId: perm, ...newPerm });
		}
		return { added, removed };
	}

	function isDefault(d: PermissionData): boolean {
		for (const [k, v] of Object.entries(defaultPerm)) {
			if ((d as any)[k] !== v)
				return false;
		}
		return true;
	}

	function isEqual(a: PermissionData, b: PermissionData): boolean {
		for (const k of Object.keys(defaultPerm)) {
			if ((a as any)[k] !== (b as any)[k])
				return false;
		}
		return true;
	}

	function permissionsChanged() {
		permList = permissionList;
		allPermissions = {};
		for (const perm of permissions) {
			allPermissions[perm.permissionId] = { ...perm };
		}

		for (const [perm, _desc] of permissionList) {
			if (!(perm in allPermissions)) {
				// Default value
				allPermissions[perm] = { ...defaultPerm };
			}
		}
	}

	function resetPerm(perm: Permission) {
		allPermissions[perm] = { ...defaultPerm };
	}

	function showWithFilter(filter: string, perm: Permission, name: string, description: string | undefined): boolean {
		return filter === "" || perm.toString().includes(filter) || name.includes(filter) || (description?.includes(filter) ?? false);
	}

	// TODO Add headers for permission groups
</script>

<table class="table">
	<thead>
		<tr>
			<th>
				<p class="control has-icons-right">
					<input class="input" type="text" placeholder="Search" bind:value={filter} />
					<span class="icon is-small is-right">
						<i class="mdi mdi-magnify" />
					</span>
				</p>
			</th>
			<th>Value</th>
			<th></th>
		</tr>
	</thead>
	<tbody>
		{#each permList as [perm, data] (perm)}
			{#if showWithFilter(filter, perm, data.name, data.description)}
				<tr>
					<td>
						<label
							for={"perm" + perm}
							title="{data.name} ({perm})">
							<HighlightString {filter} content={data.description ?? data.name} />
						</label>
					</td>
					<td>
						<input
							id={"perm" + perm}
							class="input"
							type="number"
							bind:value={allPermissions[perm].permissionValue} />
					</td>
					<td>
						<button on:click={() => resetPerm(perm)}><Icon name="undo" /></button>
					</td>
				</tr>
			{/if}
		{/each}
	</tbody>
</table>

<style lang="scss">
	thead tr th {
		vertical-align: middle;
	}
</style>

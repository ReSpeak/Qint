import { Permission } from "../../ts";

export interface PermissionData {
	permissionValue: number;
}

export type InPermissionData = PermissionData & {
	permissionId: number;
};

export interface PermissionDiff {
	// Also contains changed permissions
	added: InPermissionData[];
	removed: Permission[];
}

export const defaultPerm = {
	permissionValue: 0,
};

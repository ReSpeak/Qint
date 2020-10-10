import { Connection } from "../connection";
import { writable, Writable } from "svelte/store";
import { backend } from "../backend/backend";

export type IconSource = { icon: number | undefined } | undefined;
export type IconSourceLike = {
	icon: number | undefined | null,
	avatar_hash?: string,
	uid?: number[],
	getAvatarUid?: () => string | undefined,
};

export const DummyStore: Writable<IconSource> = writable(undefined);

// TODO Rework 'serverFileSrc' and 'cacheFileSrc' once we understand how tauri works.

export function getClientIconPath(client: IconSourceLike | null | undefined, connection?: Connection, server?: string): string | undefined {
	if (!connection && !server) {
		console.error("ClientIcon needs either connection or server");
		return;
	}
	if (!client)
		return;

	if (connection) {
		if (client.avatar_hash && client.avatar_hash !== "" && client.uid)
			return `${connection.backend.serverFileSrc}/file/0/avatar_${client.getAvatarUid!()}?hash=${client.avatar_hash}`;
		else if (client.icon)
			return `${connection.backend.serverFileSrc}/file/0/icon_${client.icon}`;
	} else if (server) {
		if (client.avatar_hash && client.avatar_hash !== "" && client.uid)
			return `${backend.cacheFileSrc}/${server}/0/avatar_${client.getAvatarUid!()}`;
		else if (client.icon)
			return `${backend.cacheFileSrc}/${server}/0/icon_${client.icon}`;
	}
	return;
}

export function getClientAvatarPath(client: IconSourceLike | null | undefined, connection?: Connection, server?: string): string | undefined {
	if (!connection && !server) {
		console.error("ClientIcon needs either connection or server");
		return;
	}
	if (!client)
		return;

	if (connection) {
		if (client.avatar_hash !== "" && client.uid)
			return `${connection.backend.serverFileSrc}/file/0/avatar_${client.getAvatarUid!()}?hash=${client.avatar_hash}`;
	} else if (server) {
		if (client.avatar_hash !== "" && client.uid)
			return `${backend.cacheFileSrc}/${server}/0/avatar_${client.getAvatarUid!()}`;
	}
	return;
}

export function getIconPath(source: IconSource, connection?: Connection, server?: string): string | undefined {
	if (!connection && !server) {
		console.error("ClientIcon needs either connection or server");
		return;
	}
	if (!source || !source.icon)
		return;

	const i = source.icon;
	/**/ if (i === 100) return "alpha-c-circle-outline";
	else if (i === 200) return "alpha-o-circle-outline";
	else if (i === 300) return "alpha-s-circle-outline";
	else if (i === 500) return "alpha-q-circle-outline";
	else if (i === 600) return "alpha-v-circle-outline";

	if (connection) {
		return `${connection.backend.serverFileSrc}/file/0/icon_${i}`;
	} else if (server) {
		return `${backend.cacheFileSrc}/${server}/0/icon_${i}`;
	}
	return;
}

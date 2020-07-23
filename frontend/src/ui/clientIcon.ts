import { Connection } from "../connection";
import { BASE_ADDRESS } from "../util";
import { writable, Writable } from "svelte/store";

export type IconSource = { icon_id: number | undefined } | undefined;
type IconSourceLike = {
	icon_id: number | undefined
	avatar_hash?: string
	uid?: number[],
	getAvatarUid?: () => string | undefined,
};

export const DummyStore: Writable<IconSource> = writable(undefined);

export function getClientIconPath(client: IconSourceLike, connection?: Connection, server?: string): string | undefined {
	if (!connection && !server) {
		console.error("ClientIcon needs either connection or server");
		return;
	}
	if (!client)
		return;

	if (connection) {
		if (client.avatar_hash !== "" && client.uid)
			return `${BASE_ADDRESS}/con/${connection.guid}/file/0/avatar_${client.getAvatarUid!()}?hash=${client.avatar_hash}`;
		else if (client.icon_id !== 0)
			return `${BASE_ADDRESS}/con/${connection.guid}/file/0/icon_${client.icon_id}`;
	} else if (server) {
		if (client.avatar_hash !== "" && client.uid)
			return `${BASE_ADDRESS}/filecache/${server}/0/avatar_${client.getAvatarUid!()}`;
		else if (client.icon_id !== 0)
			return `${BASE_ADDRESS}/filecache/${server}/0/icon_${client.icon_id}`;
	}
	return;
}

export function getClientAvatarPath(client: IconSourceLike, connection?: Connection, server?: string): string | undefined {
	if (!connection && !server) {
		console.error("ClientIcon needs either connection or server");
		return;
	}
	if (!client)
		return;

	if (connection) {
		if (client.avatar_hash !== "" && client.uid)
			return `${BASE_ADDRESS}/con/${connection.guid}/file/0/avatar_${client.getAvatarUid!()}?hash=${client.avatar_hash}`;
	} else if (server) {
		if (client.avatar_hash !== "" && client.uid)
			return `${BASE_ADDRESS}/filecache/${server}/0/avatar_${client.getAvatarUid!()}`;
	}
	return;
}

export function getIconPath(source: IconSource, connection?: Connection, server?: string): string | undefined {
	if (!connection && !server) {
		console.error("ClientIcon needs either connection or server");
		return;
	}
	if (!source || source.icon_id === 0)
		return;

	const i = source.icon_id;
	/**/ if (i === 100) return "alpha-c-circle-outline";
	else if (i === 200) return "alpha-o-circle-outline";
	else if (i === 300) return "alpha-s-circle-outline";
	else if (i === 500) return "alpha-q-circle-outline";
	else if (i === 600) return "alpha-v-circle-outline";

	if (connection) {
		return `${BASE_ADDRESS}/con/${connection.guid}/file/0/icon_${source.icon_id}`;
	} else if (server) {
		return `${BASE_ADDRESS}/filecache/${server}/0/icon_${source.icon_id}`;
	}
	return;
}

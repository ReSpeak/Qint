import { Connection } from "../connection";
import { GraphQlClient } from "../tree/book";
import { BASE_ADDRESS } from "../util";

export function getClientIconPath(client: GraphQlClient, connection?: Connection, server?: string): string | undefined {
	if (!connection && !server) {
		console.error("ClientIcon needs either connection or server");
		return;
	}
	if (!client)
		return;

	if (connection) {
		if (client.avatar_hash !== "" && client.uid)
			return `${BASE_ADDRESS}/con/${connection.guid}/file/0/avatar_${client.getAvatarUid()}?hash=${client.avatar_hash}`;
		else if (client.icon_id !== 0)
			return `${BASE_ADDRESS}/con/${connection.guid}/file/0/icon_${client.icon_id}`;
	}
	server = server ?? connection?.server;
	if (server) {
		if (client.avatar_hash !== "" && client.uid)
			return `${BASE_ADDRESS}/filecache/${server}/0/avatar_${client.getAvatarUid()}`;
		else if (client.icon_id !== 0)
			return `${BASE_ADDRESS}/filecache/${server}/0/icon_${client.icon_id}`;
	}
	return;
}

export function getClientAvatarPath(client: GraphQlClient, connection?: Connection, server?: string): string | undefined {
	if (!connection && !server) {
		console.error("ClientIcon needs either connection or server");
		return;
	}
	if (!client)
		return;

	if (connection) {
		if (client.avatar_hash !== "" && client.uid)
			return `${BASE_ADDRESS}/con/${connection.guid}/file/0/avatar_${client.getAvatarUid()}?hash=${client.avatar_hash}`;
	}
	server = server ?? connection?.server;
	if (server) {
		if (client.avatar_hash !== "" && client.uid)
			return `${BASE_ADDRESS}/filecache/${server}/0/avatar_${client.getAvatarUid()}`;
	}
	return;
}

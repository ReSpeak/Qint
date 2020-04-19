import { Connection } from "../connection";
import { Client, GraphQlClient } from "../tree/book";
import { BASE_ADDRESS } from "../util";

export default function getClientIconPath(client: GraphQlClient | Client, connection?: Connection, server?: string): string | undefined {
	if (!connection && !server) {
		console.error("ClientIcon needs either connection or server");
		return;
	}
	if (!client)
		return;

	if (connection) {
		if (client.avatar_hash !== "" && client.uid)
			return `${BASE_ADDRESS}/con/${connection.guid}/file/0/avatar_${client.getAvatarUid()}`;
		else if (client.icon_id !== 0)
			return `${BASE_ADDRESS}/con/${connection.guid}/file/0/icon_${client.icon_id}`;
	} else if (server) {
		if (client.avatar_hash !== "" && client.uid)
			return `${BASE_ADDRESS}/filecache/${server}/0/avatar_${client.getAvatarUid()}`;
		else if (client.icon_id !== 0)
			return `${BASE_ADDRESS}/filecache/${server}/0/icon_${client.icon_id}`;
	}
	return;
}
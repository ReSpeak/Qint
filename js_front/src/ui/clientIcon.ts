import { Connection } from "../connection";
import { Client } from "../tree/book";

export default function getClientIconPath(client: Client, connection?: Connection, server?: string): string | undefined {
	if (!connection && !server) {
		console.error("ClientIcon needs either connection or server");
		return;
	}
	if (!client)
		return;

	if (connection) {
		if (client.avatar && client.uid)
			return `http://localhost:4422/con/${connection.guid}/file/0/avatar_${client.getAvatarUid()}`;
		else if (client.icon)
			return `http://localhost:4422/con/${connection.guid}/file/0/icon_${client.icon}`;
	} else if (server) {
		if (client.avatar && client.uid)
			return `http://localhost:4422/filecache/${server}/0/avatar_${client.getAvatarUid()}`;
		else if (client.icon)
			return `http://localhost:4422/filecache/${server}/0/icon_${client.icon}`;
	}
	return;
}
import { IConnection } from "../../connection";
import { writable, Writable } from "svelte/store";
import { Uid } from "../../ts";

export type IconSource = { icon: string | undefined | null } | undefined;
export type IconSourceLike = {
	icon: string | undefined | null;
	avatarHash?: string;
	uid?: Uid | null;
	getAvatarUid?: () => string | undefined;
};

export const DummyStore: Writable<IconSource> = writable(undefined);

export async function getClientIconPath(
	connection: IConnection,
	client: IconSourceLike | null | undefined,
): Promise<string | undefined> {
	if (!client)
		return undefined;

	if (client.avatarHash && client.uid)
		return getClientAvatarPath(connection, client);
	else if (client.icon && client.icon !== "0")
		return getIconPath(connection, client);
	else
		return undefined;
}

export async function getClientAvatarPath(
	connection: IConnection,
	client: IconSourceLike | null | undefined,
): Promise<string | undefined> {
	if (!client || !client.avatarHash || !client.uid)
		return undefined;
	return await connection.fileProvider({
		channel: "0",
		path: `/avatar_${client.getAvatarUid!()}`,
		cache: true,
		hash: client.avatarHash
	});
}

export async function getIconPath(
	connection: IConnection,
	source: IconSource,
): Promise<string | undefined> {
	if (!source || !source.icon || source.icon === "0") return;

	const i = source.icon;
	/**/ if (i === "100") return "alpha-c-circle-outline";
	else if (i === "200") return "alpha-o-circle-outline";
	else if (i === "300") return "alpha-s-circle-outline";
	else if (i === "500") return "alpha-q-circle-outline";
	else if (i === "600") return "alpha-v-circle-outline";

	return await connection.fileProvider({
		channel: "0",
		path: `/icon_${i}`,
		cache: true
	});
}

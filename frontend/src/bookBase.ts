import { Writable, writable } from "svelte/store";
import {
	arraysEqual,
	base64Encode,
	Cached,
	datetimeDeserialize,
	getDataColor,
	tsHexEncode,
} from "./util";
import { Moment } from "moment";
import moment from "moment";
import { Uid } from "./ts";

export class ChatData {
	public readonly lastRead: Moment;
	public readonly unreadCount: number;

	public constructor(lastRead: Moment, unreadCount: number) {
		this.lastRead = lastRead;
		this.unreadCount = unreadCount;
	}

	public static fromGraphql(obj: {
		lastRead: number;
		timezone: number;
		unreadCount: number;
	}): ChatData {
		return new ChatData(datetimeDeserialize([obj.lastRead, obj.timezone]), obj.unreadCount);
	}

	public incrementUnread(): ChatData {
		return new ChatData(this.lastRead, this.unreadCount + 1);
	}
}

export class BookNode {
	protected readonly _store: Writable<this>;
	public readonly chat: Writable<ChatData> = writable(new ChatData(moment(), 0));
	public filterShow: boolean = true;
	public isSelected: boolean = false;

	constructor() {
		this._store = writable(this);
	}

	public update(obj: Partial<this>): this {
		Object.assign(this, obj);
		this._store.set(this);
		return this;
	}

	public updateChat(obj: Partial<ChatData>): void {
		this.chat.update((c) => Object.assign(c, obj));
	}

	public subscribe(run: (c: this) => any): () => void {
		return this._store.subscribe(run);
	}
}

export class Group {
	public update(obj: Partial<this>): this {
		return Object.assign(this, obj);
	}
}

export class ChannelBase extends BookNode {}

export class ChannelGroupBase extends Group {}

export class OptionalChannelDataBase {}

export class ClientBase extends BookNode {
	//public readonly uid: Uid | null;
	public readonly avatar_hash!: string;
	private readonly _color: Cached<Uid | null, string>;
	public get color(): string {
		return this._color.get();
	}
	private readonly _uidStr: Cached<Uid | null, string>;
	public get uidStr(): string {
		return this._uidStr.get();
	}

	protected constructor(uid?: number[], avatar_hash?: string) {
		super();
		// TODO Handle null uid
		this._color = new Cached(
			() => (this as any).uid,
			(u) => (u !== null ? getDataColor(u) : "")
		);
		this._uidStr = new Cached(
			() => (this as any).uid,
			(u) => (u !== null ? base64Encode(u) : "")
		);
		if (uid !== undefined) (this as any).uid = uid;
		if (avatar_hash !== undefined) this.avatar_hash = avatar_hash;
	}

	/**
	 * TeamSpeak uses a different encoding of the uid for fetching avatars.
	 *
	 * The raw data (base64-decoded) is encoded in hex, but instead of using
	 * [0-9a-f] with [a-p].
	 */
	public getAvatarUid(): string | undefined {
		if (this.avatar_hash === "" || (this as any).uid === null) return;
		return tsHexEncode((this as any).uid);
	}

	public equals(other: this | undefined): boolean {
		return other instanceof ClientBase && ClientBase.equals(this, other);
	}

	public static equals(first: ClientBase | undefined, second: ClientBase | undefined): boolean {
		if (first === second) return true;
		if (
			first === undefined ||
			second === undefined ||
			(first as any).uid === null ||
			(second as any).uid === null
		)
			return false;
		return arraysEqual((first as any).uid, (second as any).uid);
	}
}

export class OptionalClientDataBase {}

export class ConnectionClientDataBase {}

export class ServerBase extends BookNode {
	public readonly uid!: number[];
	private readonly _color: Cached<number[], string>;
	public get color(): string {
		return this._color.get();
	}
	private readonly _uidStr: Cached<number[], string>;
	public get uidStr(): string {
		return this._uidStr.get();
	}

	protected constructor(uid?: number[]) {
		super();
		this._color = new Cached(
			() => this.uid,
			(u) => getDataColor(u)
		);
		this._uidStr = new Cached(
			() => this.uid,
			(u) => base64Encode(u)
		);
		if (uid !== undefined) this.uid = uid;
	}

	public equals(other: this): boolean {
		return other instanceof ServerBase && this.uidStr === other.uidStr;
	}
}

export class ServerGroupBase extends Group {}

export class OptionalServerDataBase {}

export class ConnectionServerDataBase {}

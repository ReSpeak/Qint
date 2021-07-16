import { Writable, writable } from "svelte/store";
import {
	arraysEqual,
	base64Encode,
	Cached,
	datetimeDeserialize,
	getDataColor,
	tsHexEncode,
	urlBase64Encode,
} from "./util";
import { Moment } from "moment";
import moment from "moment";
import { IconId, Uid } from "./ts";

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

export abstract class BookNode {
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

export abstract class Group {
	public update(obj: Partial<this>): this {
		return Object.assign(this, obj);
	}
}

export abstract class ChannelBase extends BookNode {}

export abstract class ChannelGroupBase extends Group {}

export abstract class OptionalChannelDataBase {
	public abstract update(obj: Partial<this>): this;
}

export abstract class ClientBase extends BookNode {
	public abstract readonly uid: Uid | null;
	public abstract readonly name: string;
	public abstract readonly icon: IconId;
	public abstract readonly avatarHash: string;
	private readonly _color: Cached<Uid | null, string>;
	public get color(): string {
		return this._color.get();
	}
	private readonly _uidStr: Cached<Uid | null, string>;
	public get uidStr(): string {
		return this._uidStr.get();
	}

	protected constructor() {
		super();
		// TODO Handle null uid
		this._color = new Cached(
			() => this.uid,
			(u) => (u !== null ? getDataColor(u) : "")
		);
		this._uidStr = new Cached(
			() => this.uid,
			(u) => (u !== null ? base64Encode(u) : "")
		);
	}

	/**
	 * TeamSpeak uses a different encoding of the uid for fetching avatars.
	 *
	 * The raw data (base64-decoded) is encoded in hex, but instead of using
	 * [0-9a-f] with [a-p].
	 */
	public getAvatarUid(): string | undefined {
		if (this.avatarHash === "" || (this as any).uid === null) return;
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

export abstract class OptionalClientDataBase {
	public abstract update(obj: Partial<this>): this;
}

export abstract class ConnectionClientDataBase {
	public abstract update(obj: Partial<this>): this;
}

export abstract class ServerBase extends BookNode {
	public abstract readonly publicKey: number[];
	public abstract readonly name: string;
	public abstract readonly icon: IconId;
	// for `Server`: gets injected from msg.Connected
	// for `GraphQlServer`: get directly
	public readonly uid!: Uid;

	private readonly _color: Cached<number[], string>;
	public get color(): string {
		return this._color.get();
	}
	private readonly _uidStr: Cached<number[], string>;
	public get uidStr(): string {
		return this._uidStr.get();
	}
	private readonly _publicKeyStr: Cached<number[], string>;
	public get publicKeyStr(): string {
		return this._publicKeyStr.get();
	}

	protected constructor() {
		super();
		this._color = new Cached(
			() => this.uid,
			(u) => getDataColor(u)
		);
		this._uidStr = new Cached(
			() => this.uid,
			(u) => base64Encode(u)
		);
		this._publicKeyStr = new Cached(
			() => this.publicKey,
			(u) => urlBase64Encode(u)
		);
	}

	public equals(other: this): boolean {
		return other instanceof ServerBase && this.uidStr === other.uidStr;
	}
}

export class ServerGroupBase extends Group {}

export abstract class OptionalServerDataBase {
	public abstract update(obj: Partial<this>): this;
}

export abstract class ConnectionServerDataBase {
	public abstract update(obj: Partial<this>): this;
}

import { Moment } from "moment";
import { graphql, toDatetime } from "../graphql";
import { hexEncode, base64Decode } from "../util";

interface BookmarkChannel {
	id: string;
	fullPath: string;
}

interface BookmarkServer {
	name: string;
	uid: string;
	icon_id: number | undefined;
}

export class Bookmark {
	public id: string | undefined;
	public name: string | undefined;
	public username: string | undefined;
	public address: string | undefined;
	public bookmark: boolean | undefined;
	public lastUsed: Moment | undefined;
	public channel: BookmarkChannel | null = null;
	public server: BookmarkServer | null = null;

	constructor(content: any) {
		Object.assign(this, content);
		if (this.lastUsed)
			this.lastUsed = toDatetime(content.lastUsed, content.timezone);
		if (this.server !== null) {
			if (content.server?.icon !== undefined)
				this.server.icon_id = Number(content.server.icon);
			this.server.uid = hexEncode(base64Decode(this.server.uid));
		}
	}

	public async update(): Promise<void> {
		await graphql(`mutation UpdateBookmark($update: UpdateBookmark!) {
			updateBookmark(update: $update) { void }
		}`, {
			update: {
				id: this.id,
				name: this.name,
				username: this.username,
				bookmark: this.bookmark
			}
		});
	}

	public static async get(): Promise<Bookmark[]> {
		const bookmarks = await graphql(`query GetBookmarks {
			bookmarks {
				id
				name
				username
				address
				bookmark
				lastUsed
				timezone
				channel {
					id
					fullPath
				}
				server {
					name
					uid
					icon
				}
			}
		}`);
		return bookmarks.data.bookmarks.map((b: any) => new Bookmark(b))
	}

	public static async getRecent(): Promise<Bookmark | undefined> {
		try {
			return new Bookmark((await graphql(`query GetRecentBookmark {
			mostRecentBookmark {
				id
				name
				username
				address
				bookmark
				lastUsed
				timezone
				channel {
					id
					fullPath
				}
				server {
					name
					uid
					icon
				}
			}
		}`)).data.mostRecentBookmark);
		} catch (err) {
			console.log("Failed to get last bookmark", err);
			return undefined;
		}
	}
}

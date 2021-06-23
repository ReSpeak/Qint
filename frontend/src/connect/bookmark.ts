import { Moment } from "moment";
import { backend } from "../backend/backend";
import { datetimeDeserialize, urlBase64Encode } from "../util";

interface BookmarkChannel {
	id: string;
	fullPath: string;
}

interface BookmarkServer {
	name: string;
	publicKey: number[];
	urlBase64PublicKey: string;
	icon: string | undefined;
}

interface BookmarkIdentity {
	id: string;
	name: string;
}

export class Bookmark {
	public id: string | undefined;
	public name: string | undefined;
	public username: string | undefined;
	public address: string | undefined;
	public bookmark: boolean | undefined;
	public lastUsed: Moment | undefined;
	public identity: BookmarkIdentity | undefined;
	public channel: BookmarkChannel | null = null;
	public server: BookmarkServer | null = null;

	constructor(content: any) {
		Object.assign(this, content);
		if (this.lastUsed)
			this.lastUsed = datetimeDeserialize([content.lastUsed, content.timezone]);
		if (this.server !== null) {
			if (content.server?.icon !== undefined) this.server.icon = content.server.icon;
			this.server.urlBase64PublicKey = urlBase64Encode(this.server.publicKey);
		}
	}

	public async update(): Promise<void> {
		await backend.graphql(
			`mutation UpdateBookmark($update: UpdateBookmark!) {
			updateBookmark(update: $update) { void }
		}`,
			{
				update: {
					id: this.id,
					name: this.name,
					username: this.username,
					bookmark: this.bookmark,
				},
			}
		);
	}

	public static async get(): Promise<Bookmark[]> {
		const bookmarks = await backend.graphql(`query GetBookmarks {
			bookmarks {
				id
				name
				username
				address
				bookmark
				lastUsed
				timezone
				identity {
					id,
					name,
				}
				channel {
					id
					fullPath
				}
				server {
					name
					publicKey
					icon
				}
			}
		}`);
		return bookmarks.data.bookmarks.map((b: any) => new Bookmark(b));
	}

	public static async getRecent(): Promise<Bookmark | undefined> {
		try {
			return new Bookmark(
				(
					await backend.graphql(`query GetRecentBookmark {
			mostRecentBookmark {
				id
				name
				username
				address
				bookmark
				lastUsed
				timezone
				identity {
					id,
					name,
				}
				channel {
					id
					fullPath
				}
				server {
					name
					publicKey
					icon
				}
			}
		}`)
				).data.mostRecentBookmark
			);
		} catch (err) {
			console.log("Failed to get last bookmark", err);
			return undefined;
		}
	}
}

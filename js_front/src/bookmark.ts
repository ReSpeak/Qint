import { graphql, toDatetime } from "./graphql";

export class Bookmark {
	public static async get(): Promise<Bookmark[]> {
		const bookmarks = await graphql(`{
			bookmarks {
				id
				name
				username
				address
				bookmark
				lastUsed
				timezone
				server {
					name
				}
			}
		}`);
		return bookmarks.data.bookmarks.map((b: any) => {
			b.lastUsed = toDatetime(b.lastUsed, b.timezone);
			return b;
		})
	}
}

export function getRecent(): Promise<any> {
	return graphql(`{
			mostRecentBookmark {
				username
				address
			}
		}`);
}
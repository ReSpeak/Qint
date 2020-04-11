import { writable, Writable, derived, Readable } from "svelte/store";
import graphql from "./graphql";

export class Bookmark {
	public readonly id: Writable<string> = writable("");
	public readonly name: Writable<string | null> = writable(null);
	public readonly username: Writable<string> = writable("");
	public readonly address: Writable<string> = writable("");
	public readonly bookmark: Writable<boolean> = writable(false);

	public static async get(): Promise<Bookmark[]> {
		const bookmarks = await graphql(`{
			bookmarks {
				id
				name
				username
				address
				bookmark
				server {
					name
				}
			}
		}`);
		return bookmarks.data.bookmarks.map((b: any) => {
			const book = new Bookmark();
			return Object.assign(book, b);
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
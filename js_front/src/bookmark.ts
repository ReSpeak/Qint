import { writable, Writable, derived, Readable } from "svelte/store";
import graphql from "./graphql";

export class Bookmark {
	public readonly username: Writable<string> = writable("");
	public readonly address: Writable<string> = writable("");
}

export function getRecent(): Promise<any> {
    return graphql(`{
        mostRecentBookmark {
          username
          address
        }
      }`);
}
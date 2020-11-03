import { dataset_dev } from "svelte/internal";
import { backend } from "../backend/backend";
import { GraphQlClient } from "../book";
// TODO Move to a common place
import { Message } from "../chat/chat";
import { FetchResult } from "../ui/lazyList";
import { datetimeDeserialize } from "../util";

type GraphQlSearchResult = {
	count: number;
	results: { highlightedContent: string; message: any; }[];
};

export let EmptyFetch: FetchResult<SearchResult> = {
	items: [],
	canLoadBeforeStart: false,
	canLoadAfterEnd: false
};

export async function search(s: string, start: number = 0): Promise<SearchResults> {
	const res = await backend.graphql<{ search: GraphQlSearchResult }>(`query Search($query: String!, $start: Int!) {
			search(query: $query, start: $start) {
				count
				results {
					highlightedContent
					message {
						id
						invoker {
							client {
								uid
								name
								customName
							}
							icon
							avatar
						}
						invokerName
						content
						rendered
						status
						isPoke
						time
						timezone
					}
				}
			}
		}`, {
		query: s,
		start,
	});
	if ("data" in res) {
		const results: SearchResult[] = [];

		let id = start;
		res.data.search.results.forEach(res => {
			let client;
			const msg = res.message;
			if (msg.invoker) {
				client = GraphQlClient.fromGraphqlInvoker(msg.invoker);
			}
			const message = new Message(msg.id, client, msg.invokerName,
				msg.content, msg.rendered, datetimeDeserialize([msg.time, msg.timezone]), msg.status, msg.isPoke);
			results.push({
				id,
				message,
				highlightedContent: res.highlightedContent,
			});
			id++;
		});

		return {
			count: res.data.search.count,
			results,
		};
	} else {
		console.error("Search result does not contain data", res);
		return { results: [], count: 0 };
	}
}

export interface SearchResults {
	results: SearchResult[];
	count: number;
}

export interface SearchResult {
	id: number,
	message: Message;
	highlightedContent: string;
}
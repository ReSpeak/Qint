import { backend } from "../backend/backend";
import { Channel, GraphQlClient, GraphQlServer } from "../book";
// TODO Move to a common place
import { Message } from "../chat/chat";
import { FetchResult } from "../ui/lazyList";
import { datetimeDeserialize, urlBase64Encode } from "../util";

type GraphQlSearchResult = {
	count: number;
	results: {
		highlightedContent: string | null;
		highlightedName: string | null;
		highlightedAddress: string | null;
		message: any | null;
		channel: any | null;
		client: any | null;
		server: any | null;
	}[];
};

export const EmptyMessageFetch: FetchResult<MessageSearchResult> = {
	items: [],
	canLoadBeforeStart: false,
	canLoadAfterEnd: false
};

export const EmptyOtherFetch: FetchResult<OtherSearchResult> = {
	items: [],
	canLoadBeforeStart: false,
	canLoadAfterEnd: false
};

export async function search(s: string, start: number = 0): Promise<SearchResults> {
	const res = await backend.graphql<{ search: GraphQlSearchResult }>(`query Search($query: String!, $start: Int!) {
			search(query: $query, start: $start) {
				count
				results {
					highlightedContent: highlightedAttribute(attribute: "content")
					highlightedName: highlightedAttribute(attribute: "name")
					highlightedAddress: highlightedAttribute(attribute: "address")

					message {
						id
						invoker {
							server {
								publicKey
							}
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

					channel {
						id
						server {
							publicKey
							uid
							name
							address
							icon
						}
						name
						icon
					}

					client {
						uid
						name
						customName
					}

					server {
						publicKey
						uid
						name
						address
						icon
					}
				}
			}
		}`, {
		query: s,
		start,
	});
	if ("data" in res) {
		const messages: MessageSearchResult[] = [];
		const others: OtherSearchResult[] = [];

		let id = start;
		res.data.search.results.forEach(res => {
			let client;
			if (res.message !== null) {
				const msg = res.message;
				if (msg.invoker) {
					client = GraphQlClient.fromGraphqlInvoker(msg.invoker);
				}
				const message = new Message(msg.id, client, msg.invokerName,
					msg.content, msg.rendered, datetimeDeserialize([msg.time, msg.timezone]), msg.status, msg.isPoke);
				const server = urlBase64Encode(msg.invoker.server.publicKey);
				messages.push({
					id,
					message,
					server,
					highlightedContent: res.highlightedContent,
				});
			} else if (res.channel !== null) {
				const channel = res.channel;
				others.push({ id, Channel: {
					channel: Channel.fromGraphql(channel),
					server: GraphQlServer.fromGraphql(channel.server),
					highlightedName: res.highlightedName,
				}})
			} else if (res.client !== null) {
				const client = res.client;
				others.push({ id, Client: {
					client: GraphQlClient.fromGraphql(client),
					highlightedName: res.highlightedName,
				}})
			} else if (res.server !== null) {
				const server = res.server;
				others.push({ id, Server: {
					server: GraphQlServer.fromGraphql(server),
					highlightedAddress: res.highlightedAddress,
					highlightedName: res.highlightedName,
				}})
			}
			id++;
		});

		return {
			count: res.data.search.count,
			messages,
			others,
		};
	} else {
		console.error("Search result does not contain data", res);
		return { messages: [], others: [], count: 0 };
	}
}

export interface SearchResults {
	messages: MessageSearchResult[];
	others: OtherSearchResult[];
	count: number;
}

export interface MessageSearchResult {
	id: number,
	message: Message;
	server: string,
	highlightedContent: string | null;
}

export interface ChannelSearchResult {
	server: GraphQlServer,
	channel: Channel;
	highlightedName: string | null;
}

export interface ClientSearchResult {
	client: GraphQlClient;
	highlightedName: string | null;
}

export interface ServerSearchResult {
	server: GraphQlServer;
	highlightedAddress: string | null;
	highlightedName: string | null;
}

export interface OtherSearchResultCommon {
	id: number,
}

export type OtherSearchResult = OtherSearchResultCommon & ({ Channel: ChannelSearchResult } | { Client: ClientSearchResult } | { Server: ServerSearchResult });

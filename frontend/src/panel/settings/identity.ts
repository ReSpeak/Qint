import { backend } from "../../backend/backend";

export interface ApiIdentity {
	readonly id: string,
	name: string,
	// Readonly
	readonly uid: number[],
	// Readonly
	readonly level: number,
}

export async function loadIdentities(): Promise<ApiIdentity[]> {
	try {
		const req = await backend.fetch("/ident/all");
		return (await req.json()) as ApiIdentity[];
	} catch(err) {
		console.log("Failed to load identities", err);
		return [];
	}
}

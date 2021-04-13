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
	const req = await backend.fetch("/ident/all");
	return (await req.json()) as ApiIdentity[];
}

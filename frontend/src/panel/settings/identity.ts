import { getDataColor } from "../../util";
import { backend } from "../../backend/backend";

export interface ApiIdentity {
	readonly id: string;
	name: string;
	readonly uid: number[];
	readonly level: number;
	readonly color: string;
}

export async function loadIdentities(): Promise<ApiIdentity[]> {
	try {
		const req = await backend.fetch("/ident/all");
		const idents = (await req.json()) as ApiIdentity[];
		idents.forEach(ident => (ident as any).color = getDataColor(ident.uid));
		return idents;
	} catch (err) {
		console.log("Failed to load identities", err);
		return [];
	}
}

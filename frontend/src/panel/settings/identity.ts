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
		const idents = await backend.identity_list("All");
		idents.forEach((ident) => ((ident as any).color = getDataColor(ident.uid)));
		return idents;
	} catch (err) {
		console.log("Failed to load identities", err);
		return [];
	}
}

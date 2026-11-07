import { writable } from "svelte/store";
import debug from "debug";
import { backend } from "./backend/backend";
import type { MatrixMessage, MatrixStatus } from "./backend/backend";
import { fnBroadcast } from "./util";
const error = debug("error:MATRIX");

const loggedOut: MatrixStatus = { loggedIn: false, rooms: [] };

/** Auth state lives in the backend, this is only a view of it. */
export const matrixStatus = writable<MatrixStatus>(loggedOut);
/** New messages from any joined room. */
export const matrixMessages = fnBroadcast<[MatrixMessage]>();

export async function refreshMatrixStatus(): Promise<void> {
	try {
		matrixStatus.set(await backend.matrix_status());
	} catch (e) {
		error("Failed to get matrix status %o", e);
	}
}

backend.on_matrix_event((ev) => {
	if (ev.type === "Status") refreshMatrixStatus();
	else matrixMessages.call(ev.data);
});
refreshMatrixStatus();

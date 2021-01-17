import { Shortcut, ShortcutAction, Tristate } from "./transientSettings";

export const actions = [
	{ value: "", text: "" },
	{ value: "Away", text: "Away" },
	{ value: "InputMute", text: "Mute Input" },
	{ value: "OutputMute", text: "Mute Output" },
];

export function valueToAction(actionName: string, actionState: Tristate): ShortcutAction | null {
	if (!actionName || !actions.map(a => a.value).includes(actionName)) return null;
	let obj: any = {};
	obj[actionName] = actionState;
	return obj;
}

export function shortcutToHotkey(shortcut: Shortcut): Hotkey {
	return {
		keycode: shortcut.keycode,
		ctrl: false,
		shift: false,
		alt: false,
		meta: false,
		action: shortcut.action,
	};
}

export function hotkeyToShortcut(hotkey: Hotkey): Shortcut | undefined {
	if (hotkey.keycode === null || hotkey.action === null) return undefined;
	return {
		keycode: hotkey.keycode,
		action: hotkey.action,
	};
}

export function actionToText(action: ShortcutAction | null): string {
	if (!action || Object.keys(action).length === 0) return "-";
	let actionData = actions.find(a => Object.keys(action)[0] === a.text);
	return actionData?.text ?? "-";
}

export function actionToName(action: ShortcutAction | null): string {
	if (!action || Object.keys(action).length === 0) return "-";
	let actionData = actions.find(a => Object.keys(action)[0] === a.value);
	return actionData?.value ?? "-";
}

export function getActionState(action: ShortcutAction | null): Tristate | null {
	if (!action || Object.values(action).length === 0) return null;
	return Object.values(action)[0];
}

export function hotkeyToString(hotkey: Hotkey) {
	let content = [];
	if (hotkey.ctrl)  content.push("Ctrl");
	if (hotkey.shift) content.push("Shift");
	if (hotkey.alt)   content.push("Alt");
	if (hotkey.meta)  content.push("Meta");
	content.push(hotkey.keycode);
	return content.join(" + ");
}

export interface Hotkey {
	keycode: string | null;
	ctrl: boolean;
	shift: boolean;
	alt: boolean;
	meta: boolean;
	action: ShortcutAction | null;
}
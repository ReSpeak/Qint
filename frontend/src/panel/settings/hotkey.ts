import { Hotkey, HotkeyAction, HotkeySubject } from "../../transientSettings";

type Option<T> = Readonly<{ value: T | ""; text: string }>;
export const hotkeySubjects: readonly Option<HotkeySubject>[] = [
	{ value: "", text: "" },
	{ value: "Away", text: "Away" },
	{ value: "InputMute", text: "Mute Input" },
	{ value: "OutputMute", text: "Mute Output" },
];

export function isHotkeyComplete(hotkey: Hotkey): boolean {
	return hotkey.keycode != null && hotkey.action != null;
}

export function buildAction(subject: HotkeySubject): HotkeyAction | null {
	if (!subject) return null;
	const obj: HotkeyAction = {};
	obj[subject] = null;
	return obj;
}

export function getActionSubject(action: HotkeyAction | null): HotkeySubject | null {
	if (!action) return null;
	return (Object.keys(action)[0] as HotkeySubject | undefined) ?? null;
}

export function hotkeyToString(hotkey: Hotkey): string {
	const content = [];
	if (hotkey._ctrl) content.push("Ctrl");
	if (hotkey._shift) content.push("Shift");
	if (hotkey._alt) content.push("Alt");
	if (hotkey._meta) content.push("Meta");
	content.push(hotkey.keycode);
	return content.join(" + ");
}

export function translateJsKeyToWindows(jsKeyCode: string): string {
	// Too lazy to map the rest, have fun with this:
	// https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/code/code_values
	// https://github.com/LiveSplit/livesplit-core/blob/master/crates/livesplit-hotkey/src/windows/key_code.rs
	if (
		[
			"ControlLeft",
			"ControlRight",
			"ShiftLeft",
			"ShiftRight",
			"AltLeft",
			"AltRight",
			"MetaLeft",
			"MetaRight",
		].includes(jsKeyCode)
	)
		return jsKeyCode;
	switch (jsKeyCode) {
		case "ScrollLock":
			return "Scroll";
	}
	if (jsKeyCode.startsWith("Digit")) return jsKeyCode.replace("Digit", "D");
	if (jsKeyCode.startsWith("Key")) return jsKeyCode.replace("Key", "");
	return jsKeyCode;
}

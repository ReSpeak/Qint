import { Hotkey, HotkeyAction, HotkeySubject } from "../../transientSettings";

export const hotkeySubjects: ReadonlyMap<HotkeySubject | null, string> = new Map([
	[null, ""],
	["Away", "Away"],
	["InputMute", "Mute Input"],
	["OutputMute", "Mute Output"],
]);

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
	if (hotkey._ctrl && !MOD_CTRL.includes(hotkey.keycode!)) content.push("Ctrl");
	if (hotkey._shift && !MOD_SHIFT.includes(hotkey.keycode!)) content.push("Shift");
	if (hotkey._alt && !MOD_ALT.includes(hotkey.keycode!)) content.push("Alt");
	if (hotkey._meta && !MOD_META.includes(hotkey.keycode!)) content.push("Meta");
	content.push(hotkey.keycode);
	return content.join(" + ");
}

// Const names are in VK name format (not the JS one)
const MOD_CTRL: readonly string[] = ["LeftControl", "RightControl"];
const MOD_SHIFT: readonly string[] = ["LeftShift", "RightShift"];
const MOD_ALT: readonly string[] = ["LeftMenu", "RightMenu"];
const MOD_META: readonly string[] = ["LeftWin", "RightWin"];
const MOD_KEYS: readonly string[] = [...MOD_CTRL, ...MOD_SHIFT, ...MOD_ALT, ...MOD_META];

export function translateJsKeyToWindows(jsKeyCode: string): string {
	// Too lazy to map the rest, have fun with this:
	// https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/code/code_values
	// https://github.com/LiveSplit/livesplit-core/blob/master/crates/livesplit-hotkey/src/windows/key_code.rs
	switch (jsKeyCode) {
		case "Backquote":
			return "Oem3";
		case "Backslash":
			return "Oem5";
		case "BracketLeft":
			return "Oem4";
		case "BracketRight":
			return "Oem6";
		case "CapsLock":
			return "Capital";
		case "Comma":
			return "OemComma";
		case "Equal":
			return "OemPlus";
		case "IntlBackslash":
			return "Oem102";
		case "Minus":
			return "OemMinus";
		case "NumpadAdd":
			return "Add";
		case "NumpadDecimal":
			return "Decimal";
		case "NumpadDivide":
			return "Divide";
		case "Enter":
		case "NumpadEnter":
			return "Return"; // Apparently same as the big enter in VK code
		case "NumpadMultiply":
			return "Multiply";
		case "NumpadSubtract":
			return "Subtract";
		case "PageDown":
			return "Next";
		case "PageUp":
			return "Prior";
		case "Period":
			return "OemPeriod";
		case "Quote":
			return "Oem7";
		case "ScrollLock":
			return "Scroll";
		case "Slash":
			return "Oem2";
		case "ContextMenu":
			return "Apps";
		case "ControlLeft":
			return "LeftControl";
		case "ControlRight":
			return "RightControl";
		case "ShiftLeft":
			return "LeftShift";
		case "ShiftRight":
			return "RightShift";
		case "AltLeft":
			return "LeftMenu";
		case "AltRight":
			return "RightMenu";
		case "MetaLeft":
			return "LeftWin";
		case "MetaRight":
			return "RightWin";
	}
	if (jsKeyCode.startsWith("Digit")) return "D" + jsKeyCode.substring(5);
	if (jsKeyCode.startsWith("Key")) return jsKeyCode.substring(3);
	if (/^Numpad\d$/.test(jsKeyCode)) return "NumPad" + jsKeyCode.substring(6);
	return jsKeyCode;
}

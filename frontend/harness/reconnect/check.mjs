// Headless check that the selected channel's chat keeps updating after a reconnect.
import { chromium } from "playwright-core";
import fs from "fs";
import path from "path";

const exe = process.env.PATH.split(":").map((p) => path.join(p, "chromium")).find(fs.existsSync);
const browser = await chromium.launch({ executablePath: exe, args: ["--no-sandbox"] });
const page = await browser.newPage();
page.on("pageerror", (e) => console.log("PAGEERR", e.message));
await page.goto("http://localhost:4501/");
await page.waitForFunction(() => window.harness);

const h = (fn) => page.evaluate((fn) => window.harness[fn](), fn);
const shown = (text) =>
	page.waitForFunction((t) => document.body.textContent.includes(t), text, { timeout: 2000 }).then(() => true, () => false);

let failures = 0;
async function expect(label, text) {
	const ok = await shown(text);
	if (!ok) failures++;
	console.log(ok ? "OK  " : "FAIL", label);
}
await h("connectSequence");
await h("selectOwnChannel");
await h("receive");
await expect("message before reconnect", "msg 1");
await h("reconnect");
await h("receive");
await expect("message after reconnect", "msg 2");
await browser.close();
console.log(failures ? `${failures} FAILED` : "ALL OK");
process.exit(failures ? 1 : 0);

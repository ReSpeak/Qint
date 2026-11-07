// Headless check that the chat list stays docked at the bottom after (re)loading history.
// Usage: start the harness dev server, then `node harness/check.mjs [query] [shot.png]`
import { chromium } from "playwright-core";
import fs from "fs";
import path from "path";

const exe = process.env.PATH.split(":").map((p) => path.join(p, "chromium")).find(fs.existsSync);
const [query = "", shot] = process.argv.slice(2);
const browser = await chromium.launch({ executablePath: exe, args: ["--no-sandbox"] });
const page = await browser.newPage({ viewport: { width: 800, height: 700 }, deviceScaleFactor: Number(process.env.DPR ?? 1) });
page.on("pageerror", (e) => console.log("PAGEERR", e.message));

// Wait past the slowest simulated image/preview so the final layout is measured
const wait = Number(new URLSearchParams(query).get("delay") ?? 800) + 2000;
let failures = 0;
async function report(label) {
	const s = await page.evaluate(() => {
		const p = document.querySelector(".lazyListView");
		return { top: p.scrollTop, max: p.scrollHeight - p.clientHeight, n: p.querySelectorAll(".lazyListElement").length };
	});
	const ok = s.max - s.top <= 1;
	if (!ok) failures++;
	console.log(ok ? "OK  " : "FAIL", label, `off=${(s.max - s.top).toFixed(1)}px`, JSON.stringify(s));
}

await page.goto("http://localhost:4500/" + query);
await page.waitForTimeout(wait);
await report("initial load");
for (let ch = 2; ch <= 8; ch++) {
	await page.evaluate((ch) => window.harness.switchChannel(ch), ch);
	await page.waitForTimeout(wait);
	await report(`switch to ch${ch}`);
}
// A user scrolling up while late content still loads must stay undocked
await page.evaluate(() => window.harness.switchChannel(1));
await page.waitForTimeout(500);
await page.mouse.move(200, 300);
await page.mouse.wheel(0, -800);
await page.waitForTimeout(wait);
const off = await page.evaluate(() => { const p = document.querySelector(".lazyListView"); return p.scrollHeight - p.clientHeight - p.scrollTop; });
if (off < 100) failures++;
console.log(off >= 100 ? "OK  " : "FAIL", "user scroll up stays undocked", `off=${off.toFixed(1)}px`);
if (shot) await page.screenshot({ path: shot });
await browser.close();
console.log(failures ? `${failures} FAILED` : "ALL OK");
process.exit(failures ? 1 : 0);

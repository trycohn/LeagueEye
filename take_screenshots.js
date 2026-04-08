import puppeteer from 'puppeteer';
import { createServer } from 'vite';

async function run() {
  const server = await createServer({
    server: { port: 5173 },
    root: process.cwd(),
  });
  await server.listen();

  const browser = await puppeteer.launch({
    headless: "new",
    args: ['--no-sandbox', '--disable-setuid-sandbox']
  });
  const page = await browser.newPage();
  await page.setViewport({ width: 1280, height: 800 });

  // Mock Tauri API so the page doesn't crash or hang
  await page.evaluateOnNewDocument(() => {
    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd, args) => {
        if (cmd === 'poll_client_status') return false;
        if (cmd === 'get_cached_profile') return null;
        if (cmd === 'detect_account') throw new Error('Not found');
        return null;
      }
    };
  });

  await page.goto('http://localhost:5173', { waitUntil: 'networkidle0' });

  // Wait for the app to render
  await new Promise(r => setTimeout(r, 2000));

  // Click variant 4 button
  await page.evaluate(() => {
    const btns = Array.from(document.querySelectorAll('button'));
    const btn = btns.find(b => b.textContent === '4');
    if (btn) btn.click();
  });
  await new Promise(r => setTimeout(r, 1000));
  await page.screenshot({ path: 'Variant4_ProLPTracker.png' });

  // Click variant 5 button
  await page.evaluate(() => {
    const btns = Array.from(document.querySelectorAll('button'));
    const btn = btns.find(b => b.textContent === '5');
    if (btn) btn.click();
  });
  await new Promise(r => setTimeout(r, 1000));
  await page.screenshot({ path: 'Variant5_ProMastery.png' });

  // Click variant 6 button
  await page.evaluate(() => {
    const btns = Array.from(document.querySelectorAll('button'));
    const btn = btns.find(b => b.textContent === '6');
    if (btn) btn.click();
  });
  await new Promise(r => setTimeout(r, 1000));
  await page.screenshot({ path: 'Variant6_ProLocalLeaderboard.png' });

  await browser.close();
  await server.close();
  console.log('Screenshots saved: Variant4_ProLPTracker.png, Variant5_ProMastery.png, Variant6_ProLocalLeaderboard.png');
}

run().catch(console.error);

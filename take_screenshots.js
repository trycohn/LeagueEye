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

  // Click variant 7 button
  await page.evaluate(() => {
    const btns = Array.from(document.querySelectorAll('button'));
    const btn = btns.find(b => b.textContent === '7');
    if (btn) btn.click();
  });
  await new Promise(r => setTimeout(r, 1000));
  await page.screenshot({ path: 'Variant7_GlobalTracker.png' });

  // Click variant 8 button
  await page.evaluate(() => {
    const btns = Array.from(document.querySelectorAll('button'));
    const btn = btns.find(b => b.textContent === '8');
    if (btn) btn.click();
  });
  await new Promise(r => setTimeout(r, 1000));
  await page.screenshot({ path: 'Variant8_AnalyticsDashboard.png' });

  // Click variant 9 button
  await page.evaluate(() => {
    const btns = Array.from(document.querySelectorAll('button'));
    const btn = btns.find(b => b.textContent === '9');
    if (btn) btn.click();
  });
  await new Promise(r => setTimeout(r, 1000));
  await page.screenshot({ path: 'Variant9_CommunityHub.png' });

  await browser.close();
  await server.close();
  console.log('Screenshots saved: Variant7_GlobalTracker.png, Variant8_AnalyticsDashboard.png, Variant9_CommunityHub.png');
}

run().catch(console.error);

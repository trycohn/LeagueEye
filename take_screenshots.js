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

  // Variant 1 is default
  await page.screenshot({ path: 'Variant1_Hero.png' });

  // Click variant 2 button
  await page.evaluate(() => {
    const btns = Array.from(document.querySelectorAll('button'));
    const btn2 = btns.find(b => b.textContent === '2');
    if (btn2) btn2.click();
  });
  await new Promise(r => setTimeout(r, 1000));
  await page.screenshot({ path: 'Variant2_Dashboard.png' });

  // Click variant 3 button
  await page.evaluate(() => {
    const btns = Array.from(document.querySelectorAll('button'));
    const btn3 = btns.find(b => b.textContent === '3');
    if (btn3) btn3.click();
  });
  await new Promise(r => setTimeout(r, 1000));
  await page.screenshot({ path: 'Variant3_Immersive.png' });

  await browser.close();
  await server.close();
  console.log('Screenshots saved: Variant1_Hero.png, Variant2_Dashboard.png, Variant3_Immersive.png');
}

run().catch(console.error);

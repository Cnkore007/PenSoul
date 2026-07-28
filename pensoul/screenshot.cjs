const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({
    executablePath: '/Users/kimmy/Library/Caches/ms-playwright/chromium-1234/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing'
  });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto('http://localhost:5176');
  await page.waitForTimeout(2000);
  await page.screenshot({ path: 'current-state.png', fullPage: false });
  console.log('Screenshot saved');
  await browser.close();
})();

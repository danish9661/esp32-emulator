# Browser MIPS baseline via headless Chrome + webdemo GPIO run.
import re, sys
from playwright.sync_api import sync_playwright

PORT = sys.argv[1] if len(sys.argv) > 1 else '8901'
with sync_playwright() as p:
    try:
        browser = p.chromium.launch(headless=True)
    except Exception:
        browser = p.chromium.launch(headless=True, executable_path='/usr/bin/google-chrome')
    page = browser.new_page()
    msgs = []
    page.on('console', lambda m: msgs.append(f'{m.type}: {m.text[:160]}'))
    page.on('pageerror', lambda e: msgs.append(f'PAGEERROR: {str(e)[:200]}'))
    page.goto(f'http://localhost:{PORT}/', wait_until='load')
    page.wait_for_timeout(4000)
    print('title:', page.title())
    print('crossOriginIsolated:', page.evaluate('crossOriginIsolated'))
    # select GPIO demo then run
    btns = page.locator('#demoList button').all()
    print('demo buttons:', len(btns))
    names = [b.inner_text()[:28] for b in btns[:8]]
    print('first buttons:', names)
    gpio = page.locator('#demoList button', has_text='GPIO').first
    gpio.click()
    page.wait_for_timeout(1000)
    page.locator('#btnRun').click()
    # wait for the LIVE run readout (em-dash until this browser finishes a run)
    page.wait_for_function(
        "() => (document.getElementById('statLiveMips')?.innerText || '').includes('MIPS')",
        timeout=300000,
    )
    page.wait_for_timeout(2000)
    text = page.locator('#console').inner_text()
    m = re.findall(r'([\d.]+)\s*MIPS', text)
    print('MIPS readings:', m[-3:])
    print('live stat:', page.locator('#statLiveMips').inner_text())
    print('uart tail:', text[-400:])
    browser.close()

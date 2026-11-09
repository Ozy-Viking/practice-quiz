"""Capture real desktop/mobile multipart quiz and results screenshots.

Run against dx serve, e.g. uv run --with playwright==1.60.0 python docs/screenshots/capture_multipart.py
"""
import argparse
import json
from pathlib import Path
from playwright.sync_api import sync_playwright, expect

parser = argparse.ArgumentParser()
parser.add_argument('--url', default='http://127.0.0.1:8082/practice-quiz/')
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
out = Path(__file__).resolve().parent
responses = [
    'Alice and Bob each want to verify that the other participant knows K and is active in this execution.',
    'A recorded response will not match a fresh challenge. Reusing a nonce can make an old MAC usable again.',
    'Start session 1 with nonce x. Bob returns y and MAC_K(x). Start session 2 using y as the challenge, obtain MAC_K(y), then return it in session 1.',
    'Bind each MAC to a distinct direction tag, both identities, and both nonces. Check the intended role and session context.',
]
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    for name, width, height in [('desktop', 1440, 1000), ('mobile', 390, 844)]:
        page = browser.new_page(viewport={'width': width, 'height': height}, device_scale_factor=1, color_scheme='light')
        page.add_init_script("localStorage.setItem('practice-quiz-theme', 'light')")
        page.goto(args.url)
        page.wait_for_load_state('networkidle')
        page.locator('input[type=file]').set_input_files(str(root/'examples/authentication/authentication-quiz.zip'))
        page.get_by_label('Seed:', exact=True).fill('42')
        page.get_by_role('button', name='Start Quiz').click()
        expect(page.get_by_role('textbox')).to_have_count(4)
        for i, response in enumerate(responses):
            page.get_by_role('textbox').nth(i).fill(response)
        page.get_by_role('button', name='Show next hint').first.click()
        for image in page.locator('.image-enlarge img').all():
            expect(image).to_be_visible()
            assert image.evaluate('(img) => img.complete && img.naturalWidth > 0')
        assert page.evaluate('document.documentElement.scrollWidth <= innerWidth')
        page.locator('textarea').evaluate_all('(inputs) => inputs.forEach(el => el.scrollTop = 0)')
        page.evaluate('window.scrollTo(0, 0)')
        page.screenshot(path=str(out/f'multipart-{name}-quiz.png'), full_page=True)
        page.get_by_role('button', name='Submit Answers ✓').click()
        expect(page.get_by_role('heading', name='Quiz Complete')).to_be_visible()
        expect(page.locator('.short-answer-response')).to_have_count(4)
        page.evaluate('window.scrollTo(0, 0)')
        page.screenshot(path=str(out/f'multipart-{name}-results.png'), full_page=True)
        page.close()
    browser.close()
print(json.dumps({'screenshots': [str(p.relative_to(root)) for p in sorted(out.glob('multipart-*.png'))]}, indent=2))

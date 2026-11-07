import { test, expect, type Page } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import path from 'node:path';

const fixture = (name: string) => path.join(__dirname, 'fixtures/images', name);
const block = (page: Page, id: 'mc' | 'sa', results = false) => page.locator(results ? '.result-block' : '.question-block').filter({ hasText: id === 'mc' ? 'Which shape is teal?' : 'Describe the two shapes.' });
async function upload(page: Page, name = 'single.zip', drop = false) {
  if (!drop) return page.locator('input[type=file]').setInputFiles(fixture(name));
  const bytes = [...await readFile(fixture(name))];
  const data = await page.evaluateHandle(({ bytes, name }) => {
    const data = new DataTransfer();
    data.items.add(new File([new Uint8Array(bytes)], name));
    return data;
  }, { bytes, name });
  await page.locator('.upload-drop-target').dispatchEvent('drop', { dataTransfer: data });
  await data.dispose();
}
async function start(page: Page) {
  await page.getByLabel('Seed:', { exact: true }).fill('42');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
}
async function submit(page: Page) { await page.getByRole('button', { name: /Submit Answers/ }).click(); }
async function exported(page: Page) {
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.locator('.results-export [role=status]')).toContainText('copied');
  const copied = await page.evaluate(() => (window as any).copied);
  const waiting = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download Results JSON' }).click();
  const download = await waiting;
  expect(await readFile((await download.path())!, 'utf8')).toBe(copied);
  expect(copied).not.toContain('blob:');
  return JSON.parse(copied);
}
async function decoded(page: Page, results = false) {
  const images = page.locator(results ? '.result-block .image-enlarge img' : '.question-block .image-enlarge img');
  await expect(images).toHaveCount(3);
  await expect.poll(() => images.evaluateAll(images => images.every(img => (img as HTMLImageElement).naturalWidth === 320))).toBe(true);
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const state = window as any;
    state.created = []; state.revoked = [];
    const create = URL.createObjectURL.bind(URL), revoke = URL.revokeObjectURL.bind(URL);
    URL.createObjectURL = blob => { const url = create(blob); if (blob.type.startsWith('image/')) state.created.push(url); return url; };
    URL.revokeObjectURL = url => { state.revoked.push(url); revoke(url); };
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async (text: string) => { state.copied = text; } } });
  });
  await page.goto('./');
});

for (const drop of [false, true]) {
  test(`bundled images render for both kinds, enlarge accessibly and export original refs; drop=${drop}`, async ({ page }) => {
    await upload(page, 'single.zip', drop);
    await start(page);
    await decoded(page);
    expect(await page.evaluate(() => (window as any).created.length)).toBe(2);
    const sa = block(page, 'sa');
    await sa.getByRole('textbox').fill('  Teal rectangle\n日本語  ');
    const enlarge = sa.getByRole('button', { name: /Enlarge image/ });
    await enlarge.focus(); await page.keyboard.press('Enter');
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Close image' })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(enlarge).toBeFocused();
    await expect(sa.getByRole('textbox')).toHaveValue('  Teal rectangle\n日本語  ');
    await expect(page.locator('.hint-text')).toHaveCount(0);
    await block(page, 'mc').getByRole('button', { name: /Rectangle/, exact: false }).filter({ hasNot: page.locator('img') }).first().click();
    await submit(page); await decoded(page, true);
    const json = await exported(page);
    expect(json).toMatchObject({ format_version: 4, source_file: 'single.zip', source_manifest: 'Biology revision.JSON' });
    const original = JSON.parse(await readFile(fixture('quiz.json'), 'utf8'));
    for (const q of json.questions) expect(q.images).toEqual(original.questions.find((item: any) => item.id === q.id).images);
    expect(json.questions.find((q: any) => q.id === 'sa')).toMatchObject({ response: '  Teal rectangle\n日本語  ', hints_viewed: false });
  });
}

test('chooser requires explicit selection, supports keyboard and invalid-file recovery', async ({ page }) => {
  await upload(page, 'chooser.zip');
  const chooser = page.getByRole('region', { name: 'Choose a quiz' });
  await expect(chooser).toBeVisible();
  await expect(chooser.getByRole('button')).toHaveText(['A revision.json', 'notes.json', 'quiz.json', 'Cancel selection']);
  await chooser.getByRole('button', { name: 'quiz.json', exact: true }).click();
  await expect(page.locator('.error-banner')).toContainText('Invalid JSON');
  await expect(chooser).toBeVisible();
  await chooser.getByRole('button', { name: 'A revision.json', exact: true }).focus();
  await page.keyboard.press('Enter'); await start(page); await decoded(page);
  await submit(page);
  expect((await exported(page)).source_manifest).toBe('A revision.json');
});

for (const drop of [false, true]) {
  test(`one enclosing folder supports deeply nested images and original export paths; drop=${drop}`, async ({ page }) => {
    await upload(page, 'wrapped.zip', drop);
    await expect(page.getByRole('region', { name: 'Choose a quiz' })).toHaveCount(0);
    await start(page); await decoded(page);
    expect(await page.evaluate(() => (window as any).created.length)).toBe(2);
    await submit(page); await decoded(page, true);
    const json = await exported(page);
    expect(json).toMatchObject({ format_version: 4, source_file: 'wrapped.zip', source_manifest: 'quiz/quiz.json' });
    const original = JSON.parse(await readFile(fixture('quiz.json'), 'utf8'));
    for (const question of original.questions) {
      for (const image of question.images) {
        if (image.src === 'shapes.png') image.src = 'images/a/b/c/d/image.png';
      }
      expect(json.questions.find((q: any) => q.id === question.id).images).toEqual(question.images);
    }
  });
}

test('wrapped chooser sorts full paths, recovers from invalid JSON and loads the selected valid quiz', async ({ page }) => {
  await upload(page, 'wrapped-chooser.zip');
  const chooser = page.getByRole('region', { name: 'Choose a quiz' });
  await expect(chooser.getByRole('button')).toHaveText([
    'quiz/A revision.JSON', 'quiz/B second.json', 'quiz/broken-images.json',
    'quiz/notes.json', 'quiz/quiz.json', 'Cancel selection',
  ]);
  expect(await page.evaluate(() => (window as any).created)).toEqual([]);
  await chooser.getByRole('button', { name: 'quiz/quiz.json', exact: true }).click();
  await expect(page.locator('.error-banner')).toContainText('Invalid JSON');
  await expect(chooser).toBeVisible();
  await chooser.getByRole('button', { name: 'quiz/B second.json', exact: true }).focus();
  await page.keyboard.press('Enter'); await start(page); await decoded(page);
  await expect(page.getByText('Which shape is orange?', { exact: true })).toBeVisible();
  await expect(page.getByText('Which shape is teal?', { exact: true })).toHaveCount(0);
  await submit(page);
  const json = await exported(page);
  expect(json).toMatchObject({ quiz_title: 'Second wrapped quiz', source_manifest: 'quiz/B second.json' });
  expect(json.questions.find((q: any) => q.id === 'mc').images[0].src).toBe('images/second.png');
});

test('wrapped chooser image failure releases provisional assets before another selection succeeds', async ({ page }) => {
  await upload(page, 'wrapped-chooser.zip');
  const chooser = page.getByRole('region', { name: 'Choose a quiz' });
  await chooser.getByRole('button', { name: 'quiz/broken-images.json', exact: true }).click();
  await expect(page.locator('.error-banner')).toContainText('missing.png');
  await expect(chooser).toBeVisible();
  expect(await page.evaluate(() => (window as any).created.length)).toBe(1);
  expect(await page.evaluate(() => (window as any).revoked.includes((window as any).created[0]))).toBe(true);
  await chooser.getByRole('button', { name: 'quiz/A revision.JSON', exact: true }).click();
  await start(page); await decoded(page);
  expect(await page.evaluate(() => (window as any).created.length)).toBe(3);
  expect(await page.evaluate(() => (window as any).created.slice(1).some((url: string) => (window as any).revoked.includes(url)))).toBe(false);
  await submit(page);
  expect((await exported(page)).source_manifest).toBe('quiz/A revision.JSON');
});

test('an invalid root manifest does not fall back to a valid wrapped quiz', async ({ page }) => {
  await upload(page, 'root-priority-invalid.zip');
  await expect(page.locator('.error-banner')).toContainText('Invalid JSON');
  await expect(page.getByRole('region', { name: 'Choose a quiz' })).toHaveCount(0);
  await expect(page.getByRole('heading', { name: 'Load Quiz File' })).toBeVisible();
  expect(await page.evaluate(() => (window as any).created)).toEqual([]);
});

test('quiz discovery stops after one enclosing folder', async ({ page }) => {
  await upload(page, 'depth-two.zip');
  await expect(page.locator('.error-banner')).toContainText('ZIP root');
  await expect(page.locator('.error-banner')).toContainText('one enclosing folder');
  expect(await page.evaluate(() => (window as any).created)).toEqual([]);
});

test('failed and canceled replacements retain assets, answers, hints and provenance; successful replacement revokes old URLs', async ({ page }) => {
  await upload(page); await start(page);
  await block(page, 'sa').getByRole('textbox').fill('Keep this answer');
  await block(page, 'sa').getByRole('button', { name: 'Show next hint' }).click();
  await submit(page);
  const original = await exported(page);
  const urls = await page.evaluate(() => [...(window as any).created]);
  for (const name of ['chooser.zip', 'wrapped-chooser.zip', 'missing.zip', 'corrupt.zip', 'nested.zip']) {
    await page.getByRole('button', { name: /Load New Quiz/ }).click();
    await upload(page, name);
    if (name.endsWith('chooser.zip')) {
      await page.getByRole('button', { name: 'Cancel selection' }).click();
      await expect(page.getByRole('region', { name: 'Choose a quiz' })).toHaveCount(0);
    } else { await expect(page.locator('.error-banner')).toBeVisible(); }
    await page.getByRole('button', { name: /Back to Results/ }).click();
    await decoded(page, true);
    expect(await exported(page)).toEqual(original);
    expect(await page.evaluate(urls => urls.some(url => (window as any).revoked.includes(url)), urls)).toBe(false);
  }
  await page.getByRole('button', { name: /Load New Quiz/ }).click();
  await upload(page, 'wrapped-chooser.zip');
  // A newer upload removes the old chooser and replaces all active state together.
  await upload(page, 'wrapped.zip');
  await start(page); await decoded(page);
  expect(await page.evaluate(urls => urls.every(url => (window as any).revoked.includes(url)), urls)).toBe(true);
  await expect(block(page, 'sa').getByRole('textbox')).toHaveValue('');
  await submit(page);
  const replacement = await exported(page);
  expect(replacement.questions.every((q: any) => !q.hints_viewed)).toBe(true);
  expect(replacement.source_manifest).toBe('quiz/quiz.json');
});

for (const shuffle of [false, true]) {
  test(`assets survive settings and rerun, shuffle=${shuffle}`, async ({ page }) => {
    await upload(page); await start(page); await decoded(page);
    const urls = await page.locator('.image-enlarge img').evaluateAll(images => images.map(img => img.getAttribute('src')).sort());
    await page.getByRole('button', { name: /Back to Settings/ }).click();
    await start(page); await decoded(page); await submit(page);
    await page.getByRole('checkbox', { name: 'Keep question and answer order' }).setChecked(!shuffle);
    await page.getByRole('button', { name: 'Rerun same questions' }).click(); await decoded(page);
    expect(await page.locator('.image-enlarge img').evaluateAll(images => images.map(img => img.getAttribute('src')).sort())).toEqual(urls);
    expect(await page.evaluate(() => (window as any).revoked)).toEqual([]);
  });
}

test('remote HTTPS images use no referrer, show failures with retry, and fit narrow screens', async ({ page }) => {
  let fail = true;
  await page.route('https://images.quiz.test/**', async route => {
    expect(route.request().headers()['referer']).toBeUndefined();
    if (fail) await route.fulfill({ status: 404, body: 'unavailable' });
    else await route.fulfill({ contentType: 'image/png', body: await readFile(fixture('shapes.png')) });
  });
  const quiz = JSON.parse(await readFile(fixture('quiz.json'), 'utf8'));
  for (const q of quiz.questions) for (const img of q.images) img.src = 'https://images.quiz.test/extensionless';
  await page.locator('input[type=file]').setInputFiles({ name: 'remote.json', mimeType: '', buffer: Buffer.from(JSON.stringify(quiz)) });
  await start(page);
  await expect(page.getByRole('button', { name: 'Retry image' })).toHaveCount(3);
  await expect(page.getByText('Image could not load:', { exact: false })).toHaveCount(3);
  fail = false;
  for (let i = 0; i < 3; i++) await page.getByRole('button', { name: 'Retry image' }).first().click();
  await decoded(page);
  await page.getByRole('button', { name: /Enlarge image:/ }).first().click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect.poll(() => page.getByRole('dialog').locator('img').evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(320);
  await page.getByRole('button', { name: 'Close image' }).click();
  await page.setViewportSize({ width: 375, height: 812 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await submit(page);
  expect((await exported(page)).source_manifest).toBeNull();
});

test('superseded image decoding releases provisional URLs and cannot overwrite newer failure', async ({ page }) => {
  await page.evaluate(() => {
    const original = HTMLImageElement.prototype.decode;
    HTMLImageElement.prototype.decode = function () {
      return new Promise<void>((resolve, reject) => { (window as any).releaseDecode = () => original.call(this).then(resolve, reject); });
    };
  });
  await upload(page);
  await expect.poll(() => page.evaluate(() => typeof (window as any).releaseDecode)).toBe('function');
  await page.locator('input[type=file]').setInputFiles({ name: 'new.json', mimeType: '', buffer: Buffer.from('{"questions":[]}') });
  await expect(page.locator('.error-banner')).toContainText('no questions');
  await page.evaluate(() => (window as any).releaseDecode());
  await expect.poll(() => page.evaluate(() => (window as any).created.every((url: string) => (window as any).revoked.includes(url)))).toBe(true);
  await expect(page.locator('.error-banner')).toContainText('no questions');
  await expect(page.getByRole('heading', { name: 'Load Quiz File' })).toBeVisible();
});

for (const name of ['mixed.zip', 'formats.zip']) {
  test(`${name} decodes actual fixtures including remote and static GIF/WebP formats`, async ({ page }) => {
    await page.route('https://images.quiz.test/**', async route => route.fulfill({ contentType: 'image/png', body: await readFile(fixture('shapes.png')) }));
    await upload(page, name); await start(page); await decoded(page);
    await submit(page);
    const json = await exported(page);
    expect(json.source_file).toBe(name);
    if (name === 'mixed.zip') {
      expect(json.questions.find((q: any) => q.id === 'mc').images[1].src).toBe('https://images.quiz.test/extensionless');
    }
  });
}

test('browser decoder rejection cleans provisional assets before retry', async ({ page }) => {
  await page.evaluate(() => {
    const decode = HTMLImageElement.prototype.decode;
    let rejectOnce = true;
    HTMLImageElement.prototype.decode = function () {
      if (rejectOnce) { rejectOnce = false; return Promise.reject(new Error('Decoder unavailable')); }
      return decode.call(this);
    };
  });
  await upload(page);
  await expect(page.locator('.error-banner')).toContainText('Cannot decode image');
  expect(await page.evaluate(() => (window as any).created.length)).toBe(1);
  expect(await page.evaluate(() => (window as any).created.every((url: string) => (window as any).revoked.includes(url)))).toBe(true);
  await upload(page); await start(page); await decoded(page);
});

for (const wrapped of [false, true]) {
  test(`canceling chooser decoding back to results drops provisional assets and preserves the completed attempt; wrapped=${wrapped}`, async ({ page }) => {
    await upload(page); await start(page); await submit(page);
    const original = await exported(page);
    await page.getByRole('button', { name: /Load New Quiz/ }).click();
    await upload(page, wrapped ? 'wrapped-chooser.zip' : 'chooser.zip');
    await page.evaluate(() => {
      HTMLImageElement.prototype.decode = function () { return new Promise<void>(() => {}); };
    });
    await page.getByRole('button', { name: wrapped ? 'quiz/A revision.JSON' : 'A revision.json', exact: true }).click();
    await expect.poll(() => page.evaluate(() => (window as any).created.length)).toBe(3);
    await page.getByRole('button', { name: /Back to Results/ }).click();
    await decoded(page, true);
    expect(await exported(page)).toEqual(original);
    await expect.poll(() => page.evaluate(() => (window as any).revoked.includes((window as any).created[2]))).toBe(true);
    expect(await page.evaluate(() => (window as any).created.slice(0, 2).some((url: string) => (window as any).revoked.includes(url)))).toBe(false);
  });
}

for (const wrapped of [false, true]) {
  test(`a chooser selection in flight cannot commit after a newer upload; wrapped=${wrapped}`, async ({ page }) => {
    await upload(page, wrapped ? 'wrapped-chooser.zip' : 'chooser.zip');
    await page.evaluate(() => {
      const original = HTMLImageElement.prototype.decode;
      HTMLImageElement.prototype.decode = function () {
        return new Promise<void>((resolve, reject) => { (window as any).releaseDecode = () => original.call(this).then(resolve, reject); });
      };
    });
    await page.getByRole('button', { name: wrapped ? 'quiz/A revision.JSON' : 'A revision.json', exact: true }).click();
    await expect.poll(() => page.evaluate(() => typeof (window as any).releaseDecode)).toBe('function');
    await upload(page, 'nested.zip');
    await expect(page.locator('.error-banner')).toContainText('ZIP root');
    await page.evaluate(() => (window as any).releaseDecode());
    await expect.poll(() => page.evaluate(() => (window as any).created.every((url: string) => (window as any).revoked.includes(url)))).toBe(true);
    await expect(page.getByRole('region', { name: 'Choose a quiz' })).toHaveCount(0);
    await expect(page.locator('.error-banner')).toContainText('ZIP root');
  });
}

test('explicit discard from settings releases all active image URLs', async ({ page }) => {
  await upload(page);
  await page.getByRole('button', { name: '← Back', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Load Quiz File' })).toBeVisible();
  expect(await page.evaluate(() => (window as any).created.every((url: string) => (window as any).revoked.includes(url)))).toBe(true);
});

for (const wrapped of [false, true]) {
  test(`Cancel selection releases a pending decoder without waiting or leaving upload; wrapped=${wrapped}`, async ({ page }) => {
    await upload(page, wrapped ? 'wrapped-chooser.zip' : 'chooser.zip');
    await page.evaluate(() => {
      HTMLImageElement.prototype.decode = function () { return new Promise<void>(() => {}); };
    });
    await page.getByRole('button', { name: wrapped ? 'quiz/A revision.JSON' : 'A revision.json', exact: true }).click();
    await expect.poll(() => page.evaluate(() => (window as any).created.length)).toBe(1);
    await page.getByRole('button', { name: 'Cancel selection' }).click();
    await expect(page.getByRole('heading', { name: 'Load Quiz File' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Choose a quiz' })).toHaveCount(0);
    expect(await page.evaluate(() => (window as any).revoked.includes((window as any).created[0]))).toBe(true);
  });
}

for (const [name, error] of [
  ['oversized-frame.gif.zip', '20 megapixels'],
  ['oversized-header.png.zip', '20 megapixels'],
  ['oversized-metadata.png.zip', 'metadata exceeds 8 MiB'],
]) {
  test(`rejects ${name} before creating browser image resources`, async ({ page }) => {
    await upload(page, name);
    await expect(page.locator('.error-banner')).toContainText(error);
    expect(await page.evaluate(() => (window as any).created)).toEqual([]);
    await expect(page.getByRole('heading', { name: 'Load Quiz File' })).toBeVisible();
    await upload(page);
    await start(page);
    await decoded(page);
  });
}

test('rejects decoded duplicate names and reads unambiguous CP437 names', async ({ page }) => {
  await upload(page, 'duplicate-decoded.zip');
  await expect(page.locator('.error-banner')).toContainText('duplicate decoded name');
  expect(await page.evaluate(() => (window as any).created)).toEqual([]);
  await upload(page, 'cp437.zip');
  await start(page);
  await expect(page.locator('.image-enlarge img')).toHaveCount(1);
  await expect.poll(() => page.locator('.image-enlarge img').evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(320);
  await submit(page);
  const json = await exported(page);
  expect(json.source_manifest).toBe('é.json');
  expect(json.questions[0].images[0].src).toBe('é.png');
});

for (const name of ['bom.json', 'bom.zip']) {
  test(`accepts UTF-8 BOM in ${name}`, async ({ page }) => {
    await upload(page, name);
    await start(page);
    await expect(page.getByText('Explain', { exact: true })).toBeVisible();
    await submit(page);
    const json = await exported(page);
    expect(json.quiz_title).toBe('BOM quiz');
    expect(json.source_manifest).toBe(name.endsWith('.zip') ? 'named.json' : null);
  });
}

for (const format of ['gif', 'webp']) {
  test(`animated ${format} imports and displays its later frame`, async ({ page }) => {
    await upload(page, `animated-${format}.zip`);
    await start(page);
    const animated = page.locator('.image-enlarge img[alt="Animation"]');
    const expected = page.locator('.image-enlarge img[alt="Expected later frame"]');
    await expect.poll(() => animated.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(320);
    await expect.poll(() => expected.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(320);
    const blueFrame = await expected.screenshot();
    // Screenshots capture the displayed animation frame; canvas drawImage can
    // instead expose only a format's default frame.
    await expect.poll(async () => (await animated.screenshot()).equals(blueFrame), { timeout: 10_000 }).toBe(true);
  });
}

test('enlarged image errors enter the shared failure and retry state', async ({ page }) => {
  await upload(page);
  await start(page);
  await decoded(page);
  await page.getByRole('button', { name: /Enlarge image:/ }).first().click();
  await page.getByRole('dialog').locator('img').dispatchEvent('error');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Retry image' })).toHaveCount(1);
  await page.getByRole('button', { name: 'Retry image' }).click();
  await decoded(page);
  await page.getByRole('button', { name: /Enlarge image:/ }).first().click();
  await expect.poll(() => page.getByRole('dialog').locator('img').evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(320);
});

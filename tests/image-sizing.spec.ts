import { expect, test, type Page } from '@playwright/test';
import { readFile } from 'node:fs/promises';

const image = (alt: string, dimensions: { width?: number; height?: number } = {}) => ({
  src: 'https://figures.example.test/diagram.png', alt, ...dimensions,
});
const flat = {
  id: 'flat', question: 'Flat diagram', correct_answers: ['Yes'], incorrect_answers: ['No'],
  images: [image('Width only', { width: 160 }), image('Height only', { height: 90 }),
    image('Both limits', { width: 200, height: 60 }), image('Automatic'),
    image('No upscaling', { width: 1600, height: 1000 })],
};
const group = {
  id: 'group', type: 'group', question: 'Shared diagram',
  images: [image('Shared sized diagram', { width: 640, height: 400 })],
  subquestions: [{
    id: 'part', type: 'short_answer', question: 'Describe the diagram',
    images: [image('Part sized diagram', { width: 120, height: 120 })],
  }],
};

async function upload(page: Page, questions: unknown[]) {
  await page.goto('./');
  await page.locator('input[type=file]').setInputFiles({
    name: 'sized.json', mimeType: 'application/json',
    buffer: Buffer.from(JSON.stringify({ questions })),
  });
}

for (const viewport of [{ width: 1440, height: 1000 }, { width: 390, height: 844 }]) {
  test(`image limits preserve proportions, centering, enlargement and exports at ${viewport.width}px`, async ({ page }) => {
    await page.setViewportSize(viewport);
    const png = await readFile('examples/authentication/images/reflection.png'); // 800 × 480
    await page.route('https://figures.example.test/**', route => route.fulfill({ contentType: 'image/png', body: png }));
    await upload(page, [flat, group]);
    await page.getByRole('button', { name: /Start Quiz/ }).click();

    const assertSizes = async (results = false) => {
      // Unanswered MCQs also appear in Study Review after submission.
      await expect(page.locator('.image-enlarge img')).toHaveCount(results ? 12 : 7);
      for (const definition of [...flat.images, ...group.images, ...group.subquestions[0].images]) {
        const images = page.locator('.image-enlarge').getByRole('img', { name: definition.alt, exact: true });
        for (const img of await images.all()) {
          await expect.poll(() => img.evaluate((el: HTMLImageElement) => el.complete && el.naturalWidth)).toBe(800);
          const box = await img.evaluate((el: HTMLImageElement) => {
            const rect = el.getBoundingClientRect();
            const container = el.closest('.question-images')!.getBoundingClientRect();
            const button = el.closest('button')!;
            const style = getComputedStyle(button);
            const inset = parseFloat(style.paddingLeft) + parseFloat(style.paddingRight)
              + parseFloat(style.borderLeftWidth) + parseFloat(style.borderRightWidth);
            return { width: rect.width, height: rect.height, available: container.width - inset,
              centerOffset: Math.abs(rect.x + rect.width / 2 - container.x - container.width / 2),
              defaultHeight: 22 * parseFloat(getComputedStyle(document.documentElement).fontSize) };
          });
          const scale = Math.min(1, (definition.width ?? 800) / 800,
            (definition.height ?? box.defaultHeight) / 480, box.available / 800);
          expect(box.width).toBeCloseTo(800 * scale, 0);
          expect(box.height).toBeCloseTo(480 * scale, 0);
          expect(box.centerOffset).toBeLessThan(2);
        }
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    };
    await assertSizes();

    // Thumbnail limits must not cap the full-size dialog image.
    await page.getByRole('button', { name: 'Enlarge image: Width only', exact: true }).click();
    const enlarged = page.getByRole('dialog').getByRole('img');
    await expect(enlarged).toBeVisible();
    expect((await enlarged.boundingBox())!.width).toBeGreaterThan(160);
    await page.getByRole('button', { name: 'Close image' }).click();

    await page.getByRole('button', { name: /Submit Answers/ }).click();
    await assertSizes(true);
    const pending = page.waitForEvent('download');
    await page.getByRole('button', { name: 'Download Results JSON' }).click();
    const download = await pending;
    const result = JSON.parse(await readFile((await download.path())!, 'utf8'));
    expect(result.format_version).toBe(5);
    expect(result.questions.find((q: any) => q.id === 'flat').images).toEqual(flat.images);
    const exportedGroup = result.questions.find((q: any) => q.id === 'group');
    expect(exportedGroup.images).toEqual(group.images);
    expect(exportedGroup.subquestions[0].images).toEqual(group.subquestions[0].images);
  });
}

test('invalid dimensions on shared and child images reject the upload', async ({ page }) => {
  for (const onParent of [true, false]) {
    const invalid = structuredClone(group);
    const target = onParent ? invalid : invalid.subquestions[0];
    target.images[0].height = 0;
    await upload(page, [invalid]);
    await expect(page.locator('.error-banner')).toBeVisible();
    await expect(page.getByRole('button', { name: /Start Quiz/ })).toHaveCount(0);
  }
});

test('schema-valid decimal and exponent dimensions load through JSON upload', async ({ page }) => {
  const png = await readFile('examples/authentication/images/reflection.png');
  await page.route('https://figures.example.test/**', route => route.fulfill({ contentType: 'image/png', body: png }));
  await page.goto('./');
  const raw = JSON.stringify({ questions: [group] })
    .replace('"width":640', '"width":6.4e2')
    .replace('"height":400', '"height":400.0');
  await page.locator('input[type=file]').setInputFiles({
    name: 'integral-dimensions.json', mimeType: 'application/json', buffer: Buffer.from(raw),
  });
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  const image = page.locator('.image-enlarge').getByRole('img', { name: 'Shared sized diagram', exact: true });
  await expect.poll(() => image.evaluate((el: HTMLImageElement) => el.complete && el.naturalWidth)).toBe(800);
  const bounds = await image.boundingBox();
  expect(bounds!.width).toBeCloseTo(640, 0);
  expect(bounds!.height).toBeCloseTo(384, 0);
});

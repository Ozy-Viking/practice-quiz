import { readFile } from 'node:fs/promises';
import { expect, test, type Page } from '@playwright/test';

const fixture = 'examples/authentication/authentication-quiz.zip';
const prompt = 'Shared protocol\nAlice and Bob share K.';
const grouped = {
  title: 'Multipart regression',
  questions: [
    { id: 'parent"<>', type: 'group', question: prompt,
      metadata: { topic: 'Parent topic', study_location: 'Parent location', timestamp: 'LECTURE-00:42', answer: 'SHARED-ANSWER-KEY' }, explanation: 'Shared explanation exactly once.',
      subquestions: [
        { id: 'a', type: 'short_answer', question: 'State the goal.', marks: 1, hints: ['Part a hint'] },
        { id: 'b', type: 'short_answer', question: 'Explain replay.', marks: 1 },
        { id: 'c', type: 'short_answer', question: 'Reflect two sessions.', marks: 2 },
        { id: 'd', type: 'short_answer', question: 'Fix the protocol.', marks: 2 },
      ] },
  ],
};
async function upload(page: Page, quiz: any = grouped) {
  await page.goto('./');
  await page.locator('input[type=file]').setInputFiles({ name: 'group.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(quiz)) });
  await expect(page.getByRole('button', { name: /Start Quiz/ })).toBeVisible();
  await page.getByLabel('Seed:', { exact: true }).fill('42');
}
async function start(page: Page, quiz: any = grouped) {
  await upload(page, quiz);
  await page.getByRole('button', { name: /Start Quiz/ }).click();
}
async function submit(page: Page) {
  await page.getByRole('button', { name: /Submit Answers/ }).click();
}
async function exported(page: Page) {
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.locator('.results-export [role=status]')).toHaveText('Results JSON copied to clipboard.');
  const copied = await page.evaluate(() => (window as any).copiedResults as string);
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download Results JSON' }).click();
  const download = await pending;
  expect(await readFile((await download.path())!, 'utf8')).toBe(copied);
  return JSON.parse(copied);
}
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async (text: string) => { (window as any).copiedResults = text; } } });
  });
});

test('four accessible independent textareas preserve edits, literal whitespace, and partial completion', async ({ page }) => {
  await start(page);
  await expect(page.getByText(/SHARED-ANSWER-KEY|LECTURE-00:42/)).toHaveCount(0);
  await expect(page.locator('.group-context')).toHaveCount(1);
  await expect(page.locator('.multipart-card')).toHaveCount(1);
  await expect(page.locator('.multipart-card > .question-block')).toHaveCount(4);
  await expect(page.locator('.multipart-card').getByRole('textbox')).toHaveCount(4);
  await expect(page.getByText(prompt, { exact: true })).toHaveCount(1);
  await expect(page.getByRole('textbox')).toHaveCount(4);
  const raw = '  Mutual authentication\n日本語 <b>literal</b>\t ';
  await page.getByRole('textbox', { name: 'Q1(a) State the goal.' }).fill(raw);
  await page.getByRole('textbox', { name: 'Q1(b) Explain replay.' }).fill('Earlier answer');
  await page.getByRole('textbox', { name: 'Q1(b) Explain replay.' }).fill(' \n\t ');
  await page.getByRole('textbox', { name: 'Q1(c) Reflect two sessions.' }).fill('Use a concurrent session.');
  await expect(page.getByText('2/4 answered')).toBeVisible();
  await expect(page.getByRole('textbox').nth(0)).toHaveValue(raw);
  await expect(page.getByRole('textbox').nth(3)).toHaveValue('');
  await page.getByRole('button', { name: 'Show next hint' }).click();
  await submit(page);
  await expect(page.getByRole('heading', { name: 'Quiz Complete' })).toBeVisible();
  await expect(page.locator('.score-banner')).toHaveCount(0);
  await expect(page.getByText('Total available marks: 6')).toBeVisible();
  await expect(page.getByText('Explanation: Shared explanation exactly once.', { exact: true })).toHaveCount(1);
  await expect(page.getByText('Timestamp: LECTURE-00:42', { exact: true })).toHaveCount(1);
  await expect(page.getByText('Original answer key: SHARED-ANSWER-KEY', { exact: true })).toHaveCount(1);
  await expect(page.locator('.result-group-context').getByText('Timestamp: LECTURE-00:42', { exact: true })).toBeVisible();
  await expect(page.locator('.result-group-context').getByText('Original answer key: SHARED-ANSWER-KEY', { exact: true })).toBeVisible();
  await expect(page.locator('.multipart-card')).toHaveCount(1);
  await expect(page.locator('.multipart-card > .result-block')).toHaveCount(4);
  const json = await exported(page);
  expect(json).toMatchObject({ format_version: 5, question_count: 1, answerable_question_count: 4, scored_question_count: 0, total_max: 0, total_available_marks: 6, percentage: null });
  const parent = json.questions[0];
  expect(parent.type).toBe('group');
  for (const field of ['status', 'score', 'response', 'hints']) expect(parent[field]).toBeUndefined();
  expect(parent.subquestions.map((q: any) => q.id)).toEqual(['a', 'b', 'c', 'd']);
  expect(parent.subquestions[0]).toMatchObject({ response: raw, hints_viewed: true, viewed_hint_indices: [0], metadata: { topic: null } });
  expect(parent.subquestions[1]).toMatchObject({ response: ' \n\t ', status: 'unanswered_unmarked' });
});

for (const shuffle of [false, true]) {
  test(`group rerun clears every response and hint, shuffle=${shuffle}`, async ({ page }) => {
    const quiz = structuredClone(grouped);
    quiz.questions.push({ id: 'flat', type: 'short_answer', question: 'Separate top-level question.' } as any);
    await start(page, quiz);
    await page.getByRole('textbox', { name: /State the goal/ }).fill('Discard me');
    await page.getByRole('button', { name: 'Show next hint' }).click();
    await submit(page);
    await page.getByRole('checkbox', { name: 'Keep question and answer order' }).setChecked(!shuffle);
    await page.getByRole('button', { name: 'Rerun same questions' }).click();
    await expect(page.getByText('0/5 answered')).toBeVisible();
    for (const textbox of await page.getByRole('textbox').all()) await expect(textbox).toHaveValue('');
    await expect(page.locator('.hint-text')).toHaveCount(0);
    const labels = await page.locator('.question-text').allTextContents();
    expect(labels.filter(t => /goal|replay|sessions|Fix/.test(t))).toEqual(['State the goal.', 'Explain replay.', 'Reflect two sessions.', 'Fix the protocol.']);
    await submit(page);
    const json = await exported(page);
    for (const q of json.questions.find((q: any) => q.type === 'group').subquestions) expect(q).toMatchObject({ response: '', hints_viewed: false, viewed_hint_indices: [] });
  });
}

test('parent metadata filters whole groups and count selects all parts', async ({ page }) => {
  const quiz = structuredClone(grouped);
  (quiz.questions[0].subquestions[0] as any).metadata = { topic: 'Child topic' } as any;
  quiz.questions.push({ id: 'other', type: 'short_answer', question: 'Excluded question', metadata: { topic: 'Other topic' } } as any);
  await upload(page, quiz);
  await expect(page.getByText(/Selecting one multipart question includes all its parts/)).toBeVisible();
  await page.getByPlaceholder('regex (e.g. Chapter [1-3])').fill('Parent topic');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  await expect(page.getByRole('textbox')).toHaveCount(4);
  await expect(page.getByText('Excluded question', { exact: true })).toHaveCount(0);
  expect(await page.locator('.question-number').allTextContents()).toEqual(['Q1 · 4 parts', 'Q1(a)', 'Q1(b)', 'Q1(c)', 'Q1(d)']);
});

test('mixed groups score MCQ leaves once and Study Review links to the correct part', async ({ page }) => {
  const quiz = structuredClone(grouped);
  quiz.questions[0].subquestions.push({ id: 'mc-part', type: 'multiple_choice', question: 'Select a secure fix.', marks: 3, correct_answers: ['Direction tag'], incorrect_answers: ['Reuse nonce'] } as any);
  quiz.questions.push({ id: 'mc-flat', question: 'True or false?', correct_answers: ['True'], incorrect_answers: ['False'] } as any);
  await start(page, quiz);
  const flat = page.locator('.question-block').filter({ has: page.getByText('True or false?', { exact: true }) });
  await flat.getByRole('button', { name: /True/ }).click();
  await page.getByRole('textbox', { name: /State the goal/ }).fill('Unmarked');
  await submit(page);
  await expect(page.locator('.score-number')).toHaveText('1.0 / 4');
  await expect(page.getByText('Total available marks: 10')).toBeVisible();
  const link = page.locator('.study-summary-question');
  await expect(link).toHaveCount(1);
  await expect(link).toContainText(/Q[12]\(e\): mc-part/);
  const href = await link.getAttribute('href');
  const target = page.locator(href!);
  await expect(target.locator('.result-question')).toHaveText('Select a secure fix.');
  await expect(target.locator('.answer-review')).toContainText('Direction tag');
  await expect(target.locator('.study-topic')).toHaveText('Topic: Parent topic');
  const json = await exported(page);
  expect(json).toMatchObject({ question_count: 2, answerable_question_count: 6, scored_question_count: 2, total_max: 4, total_score: 1 });
  expect(json.questions.find((q: any) => q.type === 'group').subquestions[4]).toMatchObject({ id: 'mc-part', type: 'multiple_choice', status: 'unanswered', score: 0, max_score: 3 });
});

for (const viewport of [{ width: 1440, height: 1000 }, { width: 390, height: 844 }]) {
  test(`shared and per-part ZIP images are centered and responsive at ${viewport.width}px`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto('./');
    await page.locator('input[type=file]').setInputFiles(fixture);
    await page.getByRole('button', { name: /Start Quiz/ }).click();
    await expect(page.getByRole('textbox')).toHaveCount(4);
    const images = page.locator('.image-enlarge img');
    await expect(images).toHaveCount(5);
    for (const image of await images.all()) {
      await expect.poll(() => image.evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth > 0)).toBe(true);
      const { imageRect, containerRect } = await image.evaluate(img => {
        const i = img.getBoundingClientRect(); const c = img.closest('.question-images')!.getBoundingClientRect();
        return { imageRect: { x: i.x, width: i.width }, containerRect: { x: c.x, width: c.width } };
      });
      expect(Math.abs(imageRect.x + imageRect.width / 2 - containerRect.x - containerRect.width / 2)).toBeLessThan(2);
      expect(imageRect.width).toBeLessThanOrEqual(containerRect.width);
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.getByRole('button', { name: /Enlarge image: Three-message/ }).click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    const centeredModal = await dialog.evaluate(el => {
      const d = el.getBoundingClientRect(); const i = el.querySelector('img')!.getBoundingClientRect();
      return Math.abs(i.x + i.width / 2 - d.x - d.width / 2);
    });
    expect(centeredModal).toBeLessThan(2);
    await page.getByRole('button', { name: 'Close image' }).click();
    await submit(page);
    await expect(page.locator('.image-enlarge img')).toHaveCount(5);
    const json = await exported(page);
    expect(json).toMatchObject({ source_file: 'authentication-quiz.zip', source_manifest: 'quiz.json', total_available_marks: 6 });
    expect(json.questions[0].images[0].src).toBe('images/protocol.png');
    expect(json.questions[0].subquestions[0].images[0].src).toBe('images/mutual.png');
  });
}

test('existing flat leaf images are centered too', async ({ page }) => {
  await page.goto('./');
  await page.locator('input[type=file]').setInputFiles('examples/images/image-practice.zip');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  const images = page.locator('.question-images');
  await expect(images.first()).toBeVisible();
  for (const container of await images.all()) expect(await container.evaluate(el => getComputedStyle(el).justifyContent)).toBe('center');
});

test('multiple images on shared context, children, and flat questions each sit on the centerline', async ({ page }) => {
  const image = await readFile('tests/fixtures/images/shapes.png');
  await page.route('https://figures.example.test/**', route => route.fulfill({ contentType: 'image/png', body: image }));
  const quiz = structuredClone(grouped) as any;
  const images = [1, 2].map(i => ({ src: `https://figures.example.test/${i}.png`, alt: `Diagram ${i}` }));
  quiz.questions[0].images = images;
  quiz.questions[0].subquestions[0].images = images;
  quiz.questions.push({ id: 'flat-images', type: 'short_answer', question: 'Flat images', images });
  await start(page, quiz);
  const assertCentered = async () => {
    const rendered = page.locator('.image-enlarge img');
    await expect(rendered).toHaveCount(6);
    for (const img of await rendered.all()) {
      await expect.poll(() => img.evaluate((el: HTMLImageElement) => el.complete && el.naturalWidth > 0)).toBe(true);
      expect(await img.evaluate(el => {
        const i = el.getBoundingClientRect(); const c = el.closest('.question-images')!.getBoundingClientRect();
        return Math.abs(i.x + i.width / 2 - c.x - c.width / 2);
      })).toBeLessThan(2);
    }
  };
  await assertCentered();
  await submit(page);
  await assertCentered();
});

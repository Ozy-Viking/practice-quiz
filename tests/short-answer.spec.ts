import { readFile } from 'node:fs/promises';
import { expect, test, type Page } from '@playwright/test';

const mixed = {
  title: 'Mixed practice', marks_per_question: 2, config: { marks_per_question: 9 },
  questions: [
    { id: 'mc', question: 'Pick the right option', type: 'multiple_choice', marks: 1.25,
      correct_answers: ['Right'], incorrect_answers: ['Wrong'], metadata: { topic: 'Choices' }, hints: ['MC first <b>literal</b>', 'MC second\nline'] },
    { id: 'sa', question: 'Explain your reasoning', type: 'short_answer', marks: 5.5,
      hints: ['SA first <script>literal</script>', 'SA second\nline'], metadata: { topic: 'Reasoning' } },
    { id: 'blank', question: 'Optional response', type: 'short_answer' },
  ],
};

async function picker(page: Page, quiz: unknown = mixed) {
  await page.locator('input[type=file]').setInputFiles({
    name: 'mixed.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(quiz)),
  });
}
async function start(page: Page, quiz: unknown = mixed) {
  await page.goto('./');
  await picker(page, quiz);
  await page.getByLabel('Seed:', { exact: true }).fill('42');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
}
async function submit(page: Page) {
  await page.getByRole('button', { name: /Submit Answers/ }).click();
}
async function exports(page: Page) {
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.locator('.results-export [role=status]')).toHaveText('Results JSON copied to clipboard.');
  const copied = await page.evaluate(() => (window as any).copiedResults as string);
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download Results JSON' }).click();
  const download = await pending;
  const downloaded = await readFile((await download.path())!, 'utf8');
  expect(downloaded).toBe(copied);
  return JSON.parse(downloaded);
}
function question(page: Page, text: string) {
  return page.locator('.question-block').filter({ has: page.getByText(text, { exact: true }) });
}
function result(page: Page, text: string) {
  return page.locator('.result-block').filter({ has: page.getByText(text, { exact: true }) });
}
async function drop(page: Page, files: { name: string; text: string; mime?: string }[], event = 'drop') {
  const transfer = await page.evaluateHandle((files) => {
    const data = new DataTransfer();
    for (const f of files) data.items.add(new File([f.text], f.name, { type: f.mime ?? '' }));
    return data;
  }, files);
  await page.locator('.upload-drop-target').dispatchEvent(event, { dataTransfer: transfer });
  await transfer.dispose();
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: async (text: string) => { (window as any).copiedResults = text; } },
    });
  });
});

test('mixed responses preserve exact Unicode, whitespace, literal markup, marks, and MCQ denominator', async ({ page }) => {
  await start(page);
  const sa = question(page, 'Explain your reasoning');
  const mc = question(page, 'Pick the right option');
  await expect(sa.getByText('Available marks: 5.5')).toBeVisible();
  await expect(mc.getByText('Available marks: 1.25')).toBeVisible();
  const raw = '  日本語 🦀\n<script>window.bad=true</script>\n  last\t ';
  await sa.getByRole('textbox').fill(raw);
  await mc.getByRole('button', { name: /Right/ }).click();
  await expect(sa.getByRole('textbox')).toHaveValue(raw);
  await expect(page.getByText('2/3 answered')).toBeVisible();
  await submit(page);
  await expect(page.getByRole('heading', { name: 'Multiple-choice Score' })).toBeVisible();
  await expect(page.locator('.score-number')).toHaveText('1.25 / 1.25');
  await expect(page.getByText('Total available marks: 8.75')).toBeVisible();
  const review = result(page, 'Explain your reasoning');
  expect(await review.locator('.short-answer-response').textContent()).toBe(raw);
  await expect(review.getByText('Answered — unmarked')).toBeVisible();
  await expect(review.locator('.result-score, .result-details')).toHaveCount(0);
  await expect(result(page, 'Optional response').getByText('Unanswered — unmarked')).toBeVisible();
  await expect(page.locator('.study-summary')).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).bad)).toBeUndefined();
  const json = await exports(page);
  expect(json).toMatchObject({ format_version: 3, question_count: 3, scored_question_count: 1,
    total_available_marks: 8.75, total_score: 1.25, total_max: 1.25, percentage: 100 });
  const answer = json.questions.find((q: any) => q.id === 'sa');
  expect(answer).toMatchObject({ type: 'short_answer', response: raw, status: 'answered_unmarked', max_score: 5.5 });
  for (const key of ['score', 'answers', 'correct', 'selected', 'missed', 'incorrect']) expect(answer).not.toHaveProperty(key);
});

test('both question types reveal hints sequentially by keyboard and pointer and retain actual history', async ({ page }) => {
  await start(page);
  const mc = question(page, 'Pick the right option');
  const sa = question(page, 'Explain your reasoning');
  await expect(question(page, 'Optional response').locator('.question-hint, .hint-toggle, .hint-text')).toHaveCount(0);
  for (const [block, prefix, literal] of [[mc, 'MC', '<b>literal</b>'], [sa, 'SA', '<script>literal</script>']] as const) {
    await expect(block.locator('.hint-text')).toHaveCount(0);
    const next = block.getByRole('button', { name: 'Show next hint', exact: true });
    await expect(next).toHaveCount(1);
    const target = await next.getAttribute('aria-controls');
    await next.focus();
    await page.keyboard.press('Enter');
    await expect(page.locator(`[id="${target}"]`)).toBeVisible();
    await expect(block.locator('.hint-text')).toHaveCount(1);
    await expect(block.locator('.hint-text')).toHaveText(`${prefix} first ${literal}`);
    if (prefix === 'MC') {
      await next.click();
    } else {
      await next.focus();
      await page.keyboard.press('Space');
    }
    await expect(block.locator('.hint-text')).toHaveText([`${prefix} first ${literal}`, `${prefix} second\nline`]);
    expect(await block.locator('.hint-text').nth(1).textContent()).toBe(`${prefix} second\nline`);
    await expect(block.locator('.hint-text').nth(1)).toHaveCSS('white-space', 'pre-wrap');
    await expect(block.locator('.hint-text').nth(0)).toBeVisible();
    await expect(block.locator('.hint-text').nth(1)).toBeVisible();
    await expect(next).toHaveCount(0);
    await expect(block.locator('.hint-text b, .hint-text script')).toHaveCount(0);
  }
  await expect(page.getByText('0/3 answered')).toBeVisible();
  await sa.getByRole('textbox').fill('Edit this');
  await sa.getByRole('textbox').fill('');
  await mc.getByRole('button', { name: /Right/ }).click();
  await mc.getByRole('button', { name: /Right/ }).click();
  await expect(page.getByText('0/3 answered')).toBeVisible();
  await expect(page.locator('.hint-text')).toHaveCount(4);
  await submit(page);
  for (const text of ['Pick the right option', 'Explain your reasoning']) {
    await expect(result(page, text).locator('.hint-usage')).toHaveText('Hints viewed: 2 of 2');
  }
  await expect(page.getByRole('button', { name: 'Show next hint' })).toHaveCount(0);
  const json = await exports(page);
  expect(json.total_score).toBe(0);
  expect(json.questions.find((q: any) => q.id === 'mc')).toMatchObject({ hints_viewed: true, viewed_hint_indices: [0, 1], status: 'unanswered' });
  expect(json.questions.find((q: any) => q.id === 'sa')).toMatchObject({ hints_viewed: true, viewed_hint_indices: [0, 1], status: 'unanswered_unmarked' });
  expect(await exports(page)).toEqual(json);
});

test('one reveal records only the first hint for both types', async ({ page }) => {
  await start(page);
  for (const text of ['Pick the right option', 'Explain your reasoning']) {
    const block = question(page, text);
    await block.getByRole('button', { name: 'Show next hint' }).click();
    await expect(block.locator('.hint-text')).toHaveCount(1);
  }
  await submit(page);
  for (const q of (await exports(page)).questions.filter((q: any) => q.id !== 'blank')) {
    expect(q).toMatchObject({ hints_viewed: true, viewed_hint_indices: [0] });
  }
});

test('rapid activations use current usage even before rerender for both types', async ({ page }) => {
  await start(page);
  for (const text of ['Pick the right option', 'Explain your reasoning']) {
    const block = question(page, text);
    await block.getByRole('button', { name: 'Show next hint' }).evaluate((button: HTMLButtonElement) => {
      button.click();
      button.click();
      button.click(); // A stale terminal activation must not record an invalid index.
    });
    await expect(block.locator('.hint-text')).toHaveCount(2);
    await expect(block.getByRole('button', { name: 'Show next hint' })).toHaveCount(0);
  }
  await submit(page);
  for (const q of (await exports(page)).questions.filter((q: any) => q.id !== 'blank')) {
    expect(q).toMatchObject({ hints_viewed: true, viewed_hint_indices: [0, 1] });
  }
});

for (const explicitEmpty of [false, true]) {
  test(`omitted or empty hints (${explicitEmpty}) render no controls or badges on either type`, async ({ page }) => {
    const quiz = { questions: mixed.questions.map((q) => {
      const { hints, ...rest } = q as any;
      return explicitEmpty ? { ...rest, hints: [] } : rest;
    }) };
    await start(page, quiz);
    await expect(page.locator('.question-hint, .hint-toggle, .hint-text')).toHaveCount(0);
    await submit(page);
    await expect(page.locator('.hint-usage')).toHaveCount(0);
    for (const q of (await exports(page)).questions) expect(q).toMatchObject({ hints: [], hints_viewed: false, viewed_hint_indices: [] });
  });
}

test('all short answers show completion without a score and preserve whitespace-only input', async ({ page }) => {
  await start(page, { questions: mixed.questions.filter(q => q.type === 'short_answer') });
  await question(page, 'Explain your reasoning').getByRole('textbox').fill(' \n\t ');
  await expect(page.getByText('0/2 answered')).toBeVisible();
  await submit(page);
  await expect(page.getByRole('heading', { name: 'Quiz Complete' })).toBeVisible();
  await expect(page.locator('.score-banner, .score-percent, .score-number, .study-summary')).toHaveCount(0);
  await expect(page.getByText('Total available marks: 6.5')).toBeVisible();
  await expect(page.getByText('Unanswered — unmarked', { exact: true })).toHaveCount(2);
  const json = await exports(page);
  expect(json).toMatchObject({ scored_question_count: 0, total_max: 0, total_score: 0, percentage: null });
  expect(json.questions.find((q: any) => q.id === 'sa').response).toBe(' \n\t ');
});

for (const shuffle of [false, true]) {
  test(`rerun clears both responses and hint history, shuffle=${shuffle}`, async ({ page }) => {
    await start(page);
    await question(page, 'Explain your reasoning').getByRole('textbox').fill('Old response');
    await question(page, 'Pick the right option').getByRole('button', { name: /Right/ }).click();
    for (const text of ['Explain your reasoning', 'Pick the right option']) {
      const next = question(page, text).getByRole('button', { name: 'Show next hint' });
      await next.click();
      await next.click();
    }
    await submit(page);
    await page.getByRole('checkbox', { name: 'Keep question and answer order' }).setChecked(!shuffle);
    await page.getByRole('button', { name: 'Rerun same questions' }).click();
    await expect(page.getByText('0/3 answered')).toBeVisible();
    await expect(question(page, 'Explain your reasoning').getByRole('textbox')).toHaveValue('');
    await expect(page.locator('.hint-text')).toHaveCount(0);
    await submit(page);
    const json = await exports(page);
    expect(json.total_available_marks).toBe(8.75);
    for (const q of json.questions) expect(q).toMatchObject({ hints_viewed: false, viewed_hint_indices: [] });
  });
}

test('settings and replacement uploads clear typed responses and usage; mixed questions filter normally', async ({ page }) => {
  await start(page);
  await question(page, 'Explain your reasoning').getByRole('textbox').fill('Discard');
  await question(page, 'Explain your reasoning').getByRole('button', { name: 'Show next hint' }).click();
  await page.getByRole('button', { name: /Back to Settings/ }).click();
  await page.getByPlaceholder('regex (e.g. Chapter [1-3])').fill('Reasoning');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  await expect(page.locator('.question-block')).toHaveCount(1);
  await expect(page.getByRole('textbox')).toHaveValue('');
  await expect(page.locator('.hint-text')).toHaveCount(0);
  await page.getByRole('textbox').fill('Another response');
  await page.getByRole('button', { name: 'Show next hint' }).click();
  await submit(page);
  await page.getByRole('button', { name: /Load New Quiz/ }).click();
  await drop(page, [{ name: 'replacement.json', text: JSON.stringify(mixed) }]);
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  await expect(page.getByText('0/3 answered')).toBeVisible();
  await expect(question(page, 'Explain your reasoning').getByRole('textbox')).toHaveValue('');
  await submit(page);
  for (const q of (await exports(page)).questions) expect(q).toMatchObject({ hints_viewed: false, viewed_hint_indices: [] });
});

test('drop highlights, prevents navigation, accepts missing MIME, and matches picker validation', async ({ page }) => {
  await page.goto('./');
  const files = [{ name: 'dropped.json', text: JSON.stringify(mixed) }];
  await drop(page, files, 'dragenter');
  await expect(page.locator('.upload-drop-target')).toHaveClass(/dragging/);
  await page.locator('.upload-drop-target').dispatchEvent('dragleave');
  await expect(page.locator('.upload-drop-target')).not.toHaveClass(/dragging/);
  await drop(page, files);
  await expect(page.getByText('Loaded: dropped.json (3 questions)')).toBeVisible();
  await expect(page).toHaveURL(/\/practice-quiz\/$/);
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  await expect(page.locator('.question-block')).toHaveCount(3);
});

for (const [label, text, message] of [
  ['malformed', '{broken', 'Invalid JSON'],
  ['invalid content', '{"questions":[]}', 'Quiz file contains no questions'],
] as const) {
  test(`rejects ${label} drops and picker files, clears highlighting, then recovers`, async ({ page }) => {
    await page.goto('./');
    await drop(page, [{ name: 'bad.json', text }], 'dragenter');
    await drop(page, [{ name: 'bad.json', text }]);
    await expect(page.locator('.error-banner')).toContainText(message);
    await expect(page.locator('.upload-drop-target')).not.toHaveClass(/dragging/);
    await page.locator('input[type=file]').setInputFiles({ name: 'bad.json', mimeType: '', buffer: Buffer.from(text) });
    await expect(page.locator('.error-banner')).toContainText(message);
    await expect(page.getByRole('heading', { name: 'Load Quiz File' })).toBeVisible();
    await picker(page);
    await expect(page.getByText('Loaded: mixed.json (3 questions)')).toBeVisible();
  });
}

test('multiple files are rejected and text drags ignored; picker remains keyboard accessible', async ({ page }) => {
  await page.goto('./');
  await drop(page, [{ name: 'one.json', text: '{}' }, { name: 'two.json', text: '{}' }]);
  await expect(page.locator('.error-banner')).toContainText('exactly one');
  const transfer = await page.evaluateHandle(() => { const data = new DataTransfer(); data.setData('text/plain', 'text'); return data; });
  await page.locator('.upload-drop-target').dispatchEvent('dragenter', { dataTransfer: transfer });
  await expect(page.locator('.upload-drop-target')).not.toHaveClass(/dragging/);
  await page.locator('.upload-drop-target').dispatchEvent('drop', { dataTransfer: transfer });
  await expect(page.locator('.error-banner')).toContainText('exactly one');
  await page.getByLabel('Drop a JSON quiz here, or choose a file').focus();
  await expect(page.locator('input[type=file]')).toBeFocused();
  await picker(page);
  await expect(page.getByText('Loaded: mixed.json (3 questions)')).toBeVisible();
  await transfer.dispose();
});

test('unreadable files report an error and allow retry', async ({ page }) => {
  await page.goto('./');
  await page.evaluate(() => {
    const original = File.prototype.text;
    File.prototype.text = function () { if (this.name === 'unreadable.json') return Promise.reject(new Error('Read denied')); return original.call(this); };
  });
  await drop(page, [{ name: 'unreadable.json', text: '{}' }]);
  await expect(page.locator('.error-banner')).toContainText('Failed to read file');
  await expect(page.locator('.upload-drop-target')).not.toHaveClass(/dragging/);
  await picker(page);
  await expect(page.getByText('Loaded: mixed.json (3 questions)')).toBeVisible();
});

test('a delayed older upload cannot overwrite a newer validation failure', async ({ page }) => {
  await page.goto('./');
  await page.evaluate(() => {
    const original = File.prototype.text;
    File.prototype.text = function () {
      if (this.name === 'slow.json') return new Promise<string>((resolve, reject) => {
        (window as any).finishOldRead = () => original.call(this).then(resolve, reject);
      });
      return original.call(this);
    };
  });
  await drop(page, [{ name: 'slow.json', text: JSON.stringify(mixed) }]);
  await expect.poll(() => page.evaluate(() => typeof (window as any).finishOldRead)).toBe('function');
  await drop(page, [{ name: 'latest.json', text: '{"questions":[]}' }]);
  await expect(page.locator('.error-banner')).toHaveText('Quiz file contains no questions');
  await page.evaluate(() => (window as any).finishOldRead());
  await expect(page.getByRole('heading', { name: 'Load Quiz File' })).toBeVisible();
  await expect(page.locator('.error-banner')).toHaveText('Quiz file contains no questions');
  await picker(page);
  await expect(page.getByText('Loaded: mixed.json (3 questions)')).toBeVisible();
});

test('mixed text and hint history survive export failures and retry', async ({ page }) => {
  await start(page);
  const sa = question(page, 'Explain your reasoning');
  const raw = '  Keep this\n日本語 <b>literal</b>  ';
  await sa.getByRole('textbox').fill(raw);
  await sa.getByRole('button', { name: 'Show next hint' }).click();
  await sa.getByRole('button', { name: 'Show next hint' }).click();
  await submit(page);
  await page.evaluate(() => {
    const write = navigator.clipboard.writeText.bind(navigator.clipboard);
    let copyFails = true;
    navigator.clipboard.writeText = async (text) => {
      if (copyFails) { copyFails = false; throw new Error('Denied'); }
      return write(text);
    };
    const create = URL.createObjectURL.bind(URL);
    let downloadFails = true;
    URL.createObjectURL = (blob) => {
      if (downloadFails) { downloadFails = false; throw new Error('Unavailable'); }
      return create(blob);
    };
  });
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.locator('.results-export [role=status]')).toContainText('Could not copy results.');
  await page.getByRole('button', { name: 'Download Results JSON' }).click();
  await expect(page.locator('.results-export [role=status]')).toContainText('Could not start the download.');
  const json = await exports(page);
  expect(json.questions.find((q: any) => q.id === 'sa')).toMatchObject({ response: raw, hints_viewed: true, viewed_hint_indices: [0, 1] });
});

for (const method of ['picker', 'drop'] as const) {
  test(`failed replacement via ${method} preserves completed responses and hint history`, async ({ page }) => {
    await start(page);
    const raw = '  Preserve this\n日本語 <b>literal</b>  ';
    const sa = question(page, 'Explain your reasoning');
    await sa.getByRole('textbox').fill(raw);
    await sa.getByRole('button', { name: 'Show next hint' }).click();
    await sa.getByRole('button', { name: 'Show next hint' }).click();
    await question(page, 'Pick the right option').getByRole('button', { name: /Right/ }).click();
    await submit(page);
    const original = await exports(page);
    // Opening replacement upload and cancelling must preserve the attempt too.
    await page.getByRole('button', { name: /Load New Quiz/ }).click();
    await page.getByRole('button', { name: /Back to Results/ }).click();
    expect(await exports(page)).toEqual(original);
    for (const text of ['{broken', '{"questions":[]}']) {
      await page.getByRole('button', { name: /Load New Quiz/ }).click();
      if (method === 'picker') {
        await page.locator('input[type=file]').setInputFiles({ name: 'bad.json', mimeType: 'application/json', buffer: Buffer.from(text) });
      } else {
        await drop(page, [{ name: 'bad.json', text }]);
      }
      await expect(page.locator('.error-banner')).toBeVisible();
      await page.getByRole('button', { name: /Back to Results/ }).click();
      expect(await result(page, 'Explain your reasoning').locator('.short-answer-response').textContent()).toBe(raw);
      expect(await exports(page)).toEqual(original);
    }
    // A valid replacement still clears the old attempt and hint history.
    await page.getByRole('button', { name: /Load New Quiz/ }).click();
    await picker(page);
    await page.getByRole('button', { name: /Start Quiz/ }).click();
    await expect(question(page, 'Explain your reasoning').getByRole('textbox')).toHaveValue('');
    await submit(page);
    for (const q of (await exports(page)).questions) {
      expect(q).toMatchObject({ hints_viewed: false, viewed_hint_indices: [] });
    }
  });
}

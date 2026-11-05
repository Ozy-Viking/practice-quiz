import { readFile } from 'node:fs/promises';
import { expect, type Page, test } from '@playwright/test';

const quizFixture = {
  title: 'Playwright Practice Quiz',
  config: {
    marks_per_question: 2,
    allow_negative_mark: false,
    default_question_count: 1,
  },
  questions: [
    {
      id: 'tf-1',
      question: 'Playwright can upload files in browser tests.',
      correct_answers: ['True'],
      incorrect_answers: ['False'],
      metadata: {
        study_location: 'Week 1',
        topic: 'Browser automation',
        explanation: 'setInputFiles attaches files to file inputs.',
      },
    },
    {
      id: 'mcq-1',
      question: 'Which command starts the Dioxus dev server?',
      correct_answers: ['dx serve'],
      incorrect_answers: ['cargo fmt', 'npm publish'],
      metadata: {
        study_location: 'Week 2',
        topic: 'Dioxus tooling',
      },
    },
  ],
};

const trueFalseQuizFixture = {
  ...quizFixture,
  questions: [quizFixture.questions[0]],
};

async function uploadQuiz(page: Page, quiz = quizFixture, name = 'quiz.json') {
  await page.goto('./');
  await page.locator('input[type="file"]').setInputFiles({
    name,
    mimeType: 'application/json',
    buffer: Buffer.from(JSON.stringify(quiz)),
  });
}

test('loads a valid quiz file and shows quiz settings', async ({ page }) => {
  await uploadQuiz(page);

  await expect(page.getByRole('heading', { name: 'Playwright Practice Quiz' })).toBeVisible();
  await expect(page.getByText('Loaded: quiz.json (2 questions)')).toBeVisible();
  await expect(page.getByText('Negative marking disabled (scores clamped at 0)')).toBeVisible();
  await expect(page.getByText('Each question is worth 2.0 marks')).toBeVisible();
  await expect(page.locator('input[type="number"]')).toHaveValue('1');
  await expect(page.getByText('of 2')).toBeVisible();
});

test('reports validation errors for invalid quiz files', async ({ page }) => {
  await uploadQuiz(
    page,
    {
      title: 'Broken Quiz',
      questions: [],
    },
    'broken.json',
  );

  await expect(page.getByRole('heading', { name: 'Load Quiz File' })).toBeVisible();
  await expect(page.getByText('Quiz file contains no questions')).toBeVisible();
});

test('filters the question pool before starting a quiz', async ({ page }) => {
  await uploadQuiz(page);

  await page.getByPlaceholder('regex (e.g. Week [1-3])').fill('Week 2');

  await expect(page.getByText('of 1')).toBeVisible();
  await page.getByRole('button', { name: /Start Quiz/ }).click();

  await expect(page.getByText('Which command starts the Dioxus dev server?')).toBeVisible();
  await expect(page.getByText('Playwright can upload files in browser tests.')).toBeHidden();
});

test('submits answers and shows scoring with study details', async ({ page }) => {
  await uploadQuiz(page, trueFalseQuizFixture, 'true-false-quiz.json');

  await page.getByRole('button', { name: /Start Quiz/ }).click();
  await expect(page.getByText('0/1 answered')).toBeVisible();

  await page.getByRole('button', { name: /True/ }).click();
  await expect(page.getByText('1/1 answered')).toBeVisible();
  await page.getByRole('button', { name: /Submit Answers/ }).click();

  await expect(page.getByRole('heading', { name: 'Final Score' })).toBeVisible();
  await expect(page.getByText('2.0 / 2')).toBeVisible();
  await expect(page.getByText('100%')).toBeVisible();
  await expect(page.getByText('Correct', { exact: true })).toBeVisible();
  await expect(page.getByText('Topic: Browser automation')).toBeVisible();
  await expect(page.getByText('Explanation: setInputFiles attaches files to file inputs.')).toBeVisible();
});

async function completeExportQuiz(page: Page) {
  await uploadQuiz(page, trueFalseQuizFixture, 'true-false-quiz.json');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  await page.getByRole('button', { name: /True/ }).click();
  await page.getByRole('button', { name: /Submit Answers/ }).click();
  await expect(page.getByRole('heading', { name: 'Final Score' })).toBeVisible();
}

test('copies and downloads the same complete results JSON', async ({ page, context, browserName }) => {
  if (browserName === 'chromium') {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  } else {
    // Other engines do not expose Playwright clipboard permission controls.
    await page.addInitScript(() => {
      Object.defineProperty(navigator, 'clipboard', {
        configurable: true,
        value: { writeText: async (text: string) => { (window as any).copiedResults = text; } },
      });
    });
  }
  await completeExportQuiz(page);
  const seed = Number((await page.locator('.session-seed').innerText()).replace('Seed: ', ''));
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.locator('.results-export').getByRole('status')).toHaveText('Results JSON copied to clipboard.');
  const copied = browserName === 'chromium'
    ? await page.evaluate(() => navigator.clipboard.readText())
    : await page.evaluate(() => (window as any).copiedResults as string);
  await page.evaluate(() => {
    const revoke = URL.revokeObjectURL.bind(URL);
    (window as any).revokedExportUrls = [];
    URL.revokeObjectURL = (url) => {
      (window as any).revokedExportUrls.push(url);
      revoke(url);
    };
  });
  const downloadEvent = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download Results JSON' }).click();
  const download = await downloadEvent;
  expect(download.suggestedFilename()).toBe('quiz-results.json');
  const downloaded = await readFile((await download.path())!, 'utf8');
  expect(downloaded).toBe(copied);
  const result = JSON.parse(downloaded);
  expect(result).toMatchObject({
    format_version: 2,
    quiz_title: trueFalseQuizFixture.title,
    source_file: 'true-false-quiz.json',
    seed,
    question_count: 1,
    total_score: 2,
    total_max: 2,
    percentage: 100,
    questions: [{
      id: 'tf-1',
      status: 'correct',
      score: 2,
      max_score: 2,
      answers: { a: 'True', b: 'False' },
      correct: ['a'],
      selected: ['a'],
      missed: [],
      incorrect: [],
      metadata: trueFalseQuizFixture.questions[0].metadata,
    }],
  });
  expect(result.questions[0]).toEqual({
    id: 'tf-1',
    text: trueFalseQuizFixture.questions[0].question,
    status: 'correct',
    score: 2,
    max_score: 2,
    answers: { a: 'True', b: 'False' },
    correct: ['a'],
    selected: ['a'],
    missed: [],
    incorrect: [],
    metadata: {
      ...trueFalseQuizFixture.questions[0].metadata,
      notes: null,
      timestamp: null,
      answer: null,
    },
  });
  await expect(page.locator('.results-export').getByRole('status')).toContainText('Download started.');
  await expect(page.locator('a[download="quiz-results.json"]')).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => (window as any).revokedExportUrls.length)).toBe(1);
});

test('exports complete answer mappings for partial answers and an unanswered shuffled rerun', async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: async (text: string) => { (window as any).copiedResults = text; } },
    });
  });
  const quiz = {
    ...trueFalseQuizFixture,
    questions: [{
      ...trueFalseQuizFixture.questions[0],
      id: 'multi-1',
      question: 'Which numbers are even?',
      correct_answers: ['2', '4'],
      incorrect_answers: ['3', '5'],
    }],
  };
  await uploadQuiz(page, quiz);
  await page.getByLabel('Seed:', { exact: true }).fill('42');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  const original = (await quizContent(page))[0];
  const answers = Object.fromEntries(original.options.map((text, index) => [String.fromCharCode(97 + index), text]));
  const labelsFor = (texts: string[]) => Object.entries(answers)
    .filter(([, text]) => texts.includes(text!))
    .map(([label]) => label);
  for (const answer of ['2', '3']) {
    await page.locator('.option-btn').filter({ has: page.getByText(answer, { exact: true }) }).click();
  }
  await page.getByRole('button', { name: /Submit Answers/ }).click();
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.locator('.results-export').getByRole('status')).toHaveText('Results JSON copied to clipboard.');
  const partial = JSON.parse(await page.evaluate(() => (window as any).copiedResults as string));
  expect(partial).toMatchObject({ format_version: 2, seed: 42, total_score: 0, total_max: 2, percentage: 0 });
  expect(partial.questions[0]).toMatchObject({
    id: 'multi-1',
    status: 'partially_correct',
    score: 0,
    max_score: 2,
    answers,
    correct: labelsFor(['2', '4']),
    selected: labelsFor(['2', '3']),
    missed: labelsFor(['4']),
    incorrect: labelsFor(['3']),
  });
  expect(Object.keys(partial.questions[0].answers)).toEqual(['a', 'b', 'c', 'd']);

  await page.getByLabel('Keep question and answer order').uncheck();
  await page.getByRole('button', { name: 'Rerun same questions' }).click();
  const rerun = (await quizContent(page))[0];
  const rerunAnswers = Object.fromEntries(rerun.options.map((text, index) => [String.fromCharCode(97 + index), text]));
  expect(rerun.options).not.toEqual(original.options);
  await page.getByRole('button', { name: /Submit Answers/ }).click();
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.locator('.results-export').getByRole('status')).toHaveText('Results JSON copied to clipboard.');
  const unanswered = JSON.parse(await page.evaluate(() => (window as any).copiedResults as string));
  const correct = Object.entries(rerunAnswers)
    .filter(([, text]) => ['2', '4'].includes(text!))
    .map(([label]) => label);
  expect(unanswered).toMatchObject({ format_version: 2, seed: 42, total_score: 0, total_max: 2, percentage: 0 });
  expect(unanswered.questions[0]).toMatchObject({
    status: 'unanswered',
    score: 0,
    max_score: 2,
    answers: rerunAnswers,
    correct,
    selected: [],
    missed: correct,
    incorrect: [],
  });
  expect(Object.keys(unanswered.questions[0].answers)).toEqual(['a', 'b', 'c', 'd']);
});

test('reports clipboard and download failures honestly and allows retry', async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: async () => { throw new Error('Permission denied'); } },
    });
    URL.createObjectURL = () => { throw new Error('Download unavailable'); };
  });
  await completeExportQuiz(page);
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.locator('.results-export').getByRole('status')).toContainText('Could not copy results.');
  await expect(page.getByRole('button', { name: 'Copy Results JSON' })).toBeEnabled();
  await page.getByRole('button', { name: 'Download Results JSON' }).click();
  await expect(page.locator('.results-export').getByRole('status')).toContainText('Could not start the download.');
  await expect(page.getByRole('button', { name: 'Download Results JSON' })).toBeEnabled();
  await page.evaluate(() => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: async () => {} },
    });
  });
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.locator('.results-export').getByRole('status')).toHaveText('Results JSON copied to clipboard.');
});

test('handles missing clipboard and cleans up failed download resources', async ({ page }) => {
  await completeExportQuiz(page);
  await page.evaluate(() => {
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: undefined });
    const createObjectURL = URL.createObjectURL.bind(URL);
    const revokeObjectURL = URL.revokeObjectURL.bind(URL);
    (window as any).exportUrls = { created: [], revoked: [] };
    URL.createObjectURL = (blob) => {
      const url = createObjectURL(blob);
      (window as any).exportUrls.created.push(url);
      return url;
    };
    URL.revokeObjectURL = (url) => {
      (window as any).exportUrls.revoked.push(url);
      revokeObjectURL(url);
    };
    const appendChild = document.body.appendChild.bind(document.body);
    document.body.appendChild = ((node: Node) => {
      if (node instanceof HTMLAnchorElement && node.download === 'quiz-results.json') {
        throw new Error('Cannot append download anchor');
      }
      return appendChild(node);
    }) as typeof document.body.appendChild;
  });
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.locator('.results-export').getByRole('status')).toContainText('Could not copy results.');
  await page.getByRole('button', { name: 'Download Results JSON' }).click();
  await expect(page.locator('.results-export').getByRole('status')).toContainText('Could not start the download.');
  await expect(page.locator('a[download="quiz-results.json"]')).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => (window as any).exportUrls.revoked.length)).toBe(1);
  expect(await page.evaluate(() => (window as any).exportUrls.created))
    .toEqual(await page.evaluate(() => (window as any).exportUrls.revoked));
});

const seededQuiz = require('./fixtures/seeded-quiz.json');
const seededOrder = require('./fixtures/seeded-order.json');

async function attachQuiz(page: Page, quiz: unknown) {
  await page.locator('input[type="file"]').setInputFiles({
    name: 'seeded.json', mimeType: 'application/json',
    buffer: Buffer.from(JSON.stringify(quiz)),
  });
  await expect(page.locator('#quiz-seed')).toBeVisible();
}

async function quizContent(page: Page) {
  return page.locator('.question-block').evaluateAll(blocks => blocks.map(block => ({
    text: block.querySelector('.question-text')!.textContent,
    options: Array.from(block.querySelectorAll('.option-text'), option => option.textContent),
  })));
}

const expectedContent = seededOrder.map(({ text, options }: { text: string; options: string[] }) => ({ text, options }));

test('seeded browser ordering matches native fixture and survives reupload', async ({ page }) => {
  await uploadQuiz(page, seededQuiz);
  await page.getByLabel('Seed:', { exact: true }).fill(' 00042 ');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  await expect(page.locator('#quiz-heading')).toBeFocused();
  expect(await quizContent(page)).toEqual(expectedContent);
  await expect(page.locator('.session-seed')).toHaveText('Seed: 42');

  for (const question of seededQuiz.questions) {
    const block = page.locator('.question-block').filter({ has: page.getByText(question.question, { exact: true }) });
    for (const answer of question.correct_answers) {
      await block.locator('.option-btn').filter({ has: page.getByText(answer, { exact: true }) }).click();
    }
  }
  await page.getByRole('button', { name: /Submit Answers/ }).click();
  await expect(page.locator('.score-number')).toHaveText('4.0 / 4');
  await expect(page.locator('.session-seed')).toHaveText('Seed: 42');
  await page.getByRole('button', { name: /Load New Quiz/ }).click();
  await attachQuiz(page, seededQuiz);
  await page.getByLabel('Seed:', { exact: true }).fill('42');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  expect(await quizContent(page)).toEqual(expectedContent);
});

test('reruns the exact session twice with cleared answers, focus and scroll', async ({ page }) => {
  await uploadQuiz(page, seededQuiz);
  await page.getByLabel('Seed:', { exact: true }).fill('42');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  for (let run = 0; run < 2; run++) {
    await page.locator('.option-btn').first().click();
    await expect(page.getByText('1/4 answered')).toBeVisible();
    await page.getByRole('button', { name: /Submit Answers/ }).click();
    await expect(page.locator('.score-number')).toBeVisible();
    await page.getByRole('button', { name: 'Rerun same questions' }).click();
    await expect(page.locator('.results-card')).toHaveCount(0);
    await expect(page.getByText('0/4 answered')).toBeVisible();
    await expect(page.locator('.option-btn.selected')).toHaveCount(0);
    await expect(page.locator('.session-seed')).toHaveText('Seed: 42');
    await expect(page.locator('#quiz-heading')).toBeFocused();
    await expect(page.locator('#quiz-heading')).toBeInViewport();
    expect(await quizContent(page)).toEqual(expectedContent);
  }
  await page.getByRole('button', { name: /Submit Answers/ }).click();
  await expect(page.locator('.score-number')).toHaveText('0.0 / 4');
  await expect(page.locator('.result-badge')).toHaveText(['Unanswered', 'Unanswered', 'Unanswered', 'Unanswered']);
});

test('restores manual filters, regex inputs, count and normalized seed through settings', async ({ page }) => {
  await uploadQuiz(page, seededQuiz);
  await page.getByLabel('Week 2', { exact: true }).uncheck();
  await expect(page.getByLabel('Numbers', { exact: true })).toBeChecked();
  await page.getByLabel('Logic', { exact: true }).uncheck();
  await page.getByPlaceholder('regex (e.g. Week [1-3])').fill('Week 1');
  await page.getByPlaceholder('regex (e.g. Chapter [1-3])').fill('Numbers|Logic');
  await page.getByLabel('Questions to attempt:').fill('2');
  await page.getByLabel('Seed:', { exact: true }).fill('00042');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  const first = await quizContent(page);
  for (let visit = 0; visit < 2; visit++) {
    if (visit === 1) await page.getByRole('button', { name: /Submit Answers/ }).click();
    await page.getByRole('button', { name: /Back to Settings/ }).click();
    await expect(page.getByLabel('Questions to attempt:')).toHaveValue('2');
    await expect(page.getByLabel('Seed:', { exact: true })).toHaveValue('42');
    await expect(page.getByPlaceholder('regex (e.g. Week [1-3])')).toHaveValue('Week 1');
    await expect(page.getByPlaceholder('regex (e.g. Chapter [1-3])')).toHaveValue('Numbers|Logic');
    await expect(page.getByLabel('Week 2', { exact: true })).not.toBeChecked();
    await expect(page.getByLabel('Logic', { exact: true })).not.toBeChecked();
    await page.getByRole('button', { name: /Start Quiz/ }).click();
    expect(await quizContent(page)).toEqual(first);
  }
});

test('validates decimal seed, recovers with New seed, and blocks an empty pool', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await uploadQuiz(page, seededQuiz);
  const seed = page.getByLabel('Seed:', { exact: true });
  const start = page.getByRole('button', { name: /Start Quiz/ });
  await expect(seed).toHaveValue(/^\d+$/);
  for (const value of ['', ' ', '-1', '+1', '1.5', '1e3', '0x10', '１２', '4294967296']) {
    await seed.fill(value);
    await expect(seed).toHaveAttribute('aria-invalid', 'true');
    await expect(page.getByRole('alert')).toContainText('Enter a whole number');
    await expect(start).toBeDisabled();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  }
  for (const value of ['0', '4294967295', ' 00042 ']) {
    await seed.fill(value);
    await expect(seed).toHaveAttribute('aria-invalid', 'false');
    await expect(start).toBeEnabled();
  }
  await seed.fill('invalid');
  await page.getByRole('button', { name: 'New seed', exact: true }).click();
  await expect(seed).toHaveValue(/^\d+$/);
  await expect(start).toBeEnabled();
  await page.getByPlaceholder('regex (e.g. Chapter [1-3])').fill('no such topic');
  await expect(page.getByText('No questions match the current filters.')).toBeVisible();
  await expect(start).toBeDisabled();
  await page.getByPlaceholder('regex (e.g. Chapter [1-3])').fill('');
  await expect(start).toBeEnabled();
  await start.click();
  await expect(page.getByText('0/4 answered')).toBeVisible();
});

test('successful new file resets filters, settings, session and results', async ({ page }) => {
  await uploadQuiz(page, seededQuiz);
  await page.getByPlaceholder('regex (e.g. Chapter [1-3])').fill('Numbers');
  await page.getByPlaceholder('regex (e.g. Week [1-3])').fill('Week 1');
  await page.getByLabel('Seed:', { exact: true }).fill('42');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  await page.locator('.option-btn').first().click();
  await page.getByRole('button', { name: /Submit Answers/ }).click();
  await page.getByRole('button', { name: /Load New Quiz/ }).click();
  await attachQuiz(page, quizFixture);
  await expect(page.getByLabel('Questions to attempt:')).toHaveValue('1');
  await expect(page.getByPlaceholder('regex (e.g. Chapter [1-3])')).toHaveValue('');
  await expect(page.getByPlaceholder('regex (e.g. Week [1-3])')).toHaveValue('');
  for (const checkbox of await page.getByRole('checkbox').all()) await expect(checkbox).toBeChecked();
  await expect(page.getByLabel('Seed:', { exact: true })).toHaveValue(/^\d+$/);
  await expect(page.locator('.results-card')).toHaveCount(0);
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  await expect(page.getByText('0/1 answered')).toBeVisible();
  await expect(page.locator('.question-block')).toHaveCount(1);
  await expect(page.locator('.option-btn.selected')).toHaveCount(0);
});


test('rerun can shuffle questions and answers while preserving scoring', async ({ page }) => {
  await uploadQuiz(page, seededQuiz);
  await expect(page.getByText('Use the same seed, file and settings to repeat question and answer order.')).toHaveCount(0);
  await page.getByLabel('Seed:', { exact: true }).fill('42');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  const original = await quizContent(page);
  await page.locator('.option-btn').first().click();
  await page.getByRole('button', { name: /Submit Answers/ }).click();
  await expect(page.getByLabel('Keep question and answer order')).toBeChecked();
  await page.getByLabel('Keep question and answer order').uncheck();
  await page.getByRole('button', { name: 'Rerun same questions' }).click();
  await expect(page.getByText('0/4 answered')).toBeVisible();
  await expect(page.locator('.option-btn.selected')).toHaveCount(0);
  const shuffled = await quizContent(page);
  expect(shuffled).not.toEqual(original);
  expect(shuffled).toHaveLength(original.length);
  expect(shuffled.map(q => q.text)).not.toEqual(original.map(q => q.text));
  for (const question of shuffled) {
    const before = original.find(q => q.text === question.text)!;
    expect([...question.options].sort()).toEqual([...before.options].sort());
    if (question.options.includes('True') && question.options.includes('False')) {
      expect(question.options).toEqual(['True', 'False']);
    } else {
      expect(question.options).not.toEqual(before.options);
    }
  }
  for (const question of seededQuiz.questions) {
    const block = page.locator('.question-block').filter({ has: page.getByText(question.question, { exact: true }) });
    for (const answer of question.correct_answers) {
      await block.locator('.option-btn').filter({ has: page.getByText(answer, { exact: true }) }).click();
    }
  }
  await page.getByRole('button', { name: /Submit Answers/ }).click();
  await expect(page.locator('.score-number')).toHaveText('4.0 / 4');
  await page.getByRole('button', { name: 'Rerun same questions' }).click();
  expect(await quizContent(page)).toEqual(shuffled);
});

test('copies only the seed in quiz and results and reports clipboard failures', async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: {
      writeText: async (text: string) => {
        if ((window as any).rejectCopy) throw new Error('Permission denied');
        (window as any).copiedSeed = text;
      },
    } });
  });
  await uploadQuiz(page, seededQuiz);
  await page.getByLabel('Seed:', { exact: true }).fill('42');
  await page.getByRole('button', { name: /Start Quiz/ }).click();
  await page.getByRole('button', { name: 'Copy seed 42', exact: true }).click();
  await expect(page.locator('.seed-copy-status')).toHaveText('Seed copied');
  expect(await page.evaluate(() => (window as any).copiedSeed)).toBe('42');
  await page.getByRole('button', { name: /Submit Answers/ }).click();
  await page.getByRole('button', { name: 'Copy seed 42', exact: true }).click();
  await expect(page.locator('.seed-copy-status')).toHaveText('Seed copied');
  await page.evaluate(() => { (window as any).rejectCopy = true; });
  await page.getByRole('button', { name: 'Copy seed 42', exact: true }).click();
  await expect(page.locator('.seed-copy-status')).toContainText("Couldn't copy seed");
  await expect(page.locator('.score-banner').getByRole('checkbox')).toHaveCount(0);
  await expect(page.locator('.rerun-actions').getByRole('checkbox')).toBeChecked();
});


for (const missingClipboard of ['undefined', 'null'] as const) {
  test(`seed copy reports unavailable clipboard (${missingClipboard}) on quiz and results`, async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(value => {
      Object.defineProperty(navigator, 'clipboard', {
        configurable: true,
        value: value === 'null' ? null : undefined,
      });
    }, missingClipboard);
    await uploadQuiz(page, seededQuiz);
    await page.getByLabel('Seed:', { exact: true }).fill('42');
    await page.getByRole('button', { name: /Start Quiz/ }).click();
    await page.getByRole('button', { name: 'Copy seed 42', exact: true }).click();
    await expect(page.locator('.seed-copy-status')).toContainText("Couldn't copy seed");
    await page.getByRole('button', { name: /Submit Answers/ }).click();
    await page.getByRole('button', { name: 'Copy seed 42', exact: true }).click();
    await expect(page.locator('.seed-copy-status')).toContainText("Couldn't copy seed");
    expect(errors).toEqual([]);
  });
}

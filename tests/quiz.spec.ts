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
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.getByRole('status')).toHaveText('Results JSON copied to clipboard.');
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
    format_version: 1,
    quiz_title: trueFalseQuizFixture.title,
    source_file: 'true-false-quiz.json',
    question_count: 1,
    total_score: 2,
    total_max: 2,
    percentage: 100,
    questions: [{
      id: 'tf-1',
      status: 'correct',
      score: 2,
      max_score: 2,
      selected_answers: [['A', 'True']],
      correct_answers: [['A', 'True']],
      missed_correct_answers: [],
      wrong_selected_answers: [],
      metadata: trueFalseQuizFixture.questions[0].metadata,
    }],
  });
  await expect(page.getByRole('status')).toContainText('Download started.');
  await expect(page.locator('a[download="quiz-results.json"]')).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => (window as any).revokedExportUrls.length)).toBe(1);
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
  await expect(page.getByRole('status')).toContainText('Could not copy results.');
  await expect(page.getByRole('button', { name: 'Copy Results JSON' })).toBeEnabled();
  await page.getByRole('button', { name: 'Download Results JSON' }).click();
  await expect(page.getByRole('status')).toContainText('Could not start the download.');
  await expect(page.getByRole('button', { name: 'Download Results JSON' })).toBeEnabled();
  await page.evaluate(() => {
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: async () => {} },
    });
  });
  await page.getByRole('button', { name: 'Copy Results JSON' }).click();
  await expect(page.getByRole('status')).toHaveText('Results JSON copied to clipboard.');
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
  await expect(page.getByRole('status')).toContainText('Could not copy results.');
  await page.getByRole('button', { name: 'Download Results JSON' }).click();
  await expect(page.getByRole('status')).toContainText('Could not start the download.');
  await expect(page.locator('a[download="quiz-results.json"]')).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => (window as any).exportUrls.revoked.length)).toBe(1);
  expect(await page.evaluate(() => (window as any).exportUrls.created))
    .toEqual(await page.evaluate(() => (window as any).exportUrls.revoked));
});

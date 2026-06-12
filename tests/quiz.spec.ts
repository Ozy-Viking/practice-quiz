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

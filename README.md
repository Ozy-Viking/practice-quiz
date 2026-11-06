# Practice Quiz

Use this link to the [Live Website](https://ozy-viking.github.io/practice-quiz/) and upload a json test and have a crack.

## Repeat a quiz

Settings include a decimal seed from 0 to 4294967295. A successful upload creates one
seed; **New seed** replaces it when you want another draw. Leading zeros and surrounding
spaces are accepted and normalized when you start. The active seed appears in the quiz
and results. Click the displayed seed to copy it.
Copied and downloaded results JSON include this value as the numeric `seed` field.

With the same source question array in the same order, filters, question count, seed,
and generator version, Start produces the same questions and answer order. Generator v1
uses ChaCha8Rng from rand_chacha 0.3.1 with rand 0.8 shuffling, freshly seeded for each
Start. True/False answers always put True first. Changing the file order, settings, or
generator version may change the result; a seed alone does not identify a quiz.

On results, **Rerun same questions** starts a blank attempt using the exact questions and
answer order from that session. Uncheck **Keep question and answer order** to shuffle those same
questions and their answers for the next attempt. True/False answers stay True first. The displayed seed
identifies the original draw. **Back to Settings** preserves filters, regex inputs,
count, and seed so you can adjust them or repeat the draw. Loading a new quiz resets
settings and attempts after the file passes validation. Settings and attempts stay in
memory for the current page only.

## Results JSON

Copy Results JSON and Download Results JSON export the same `format_version: 3` document.
It includes the quiz title, source filename, seed, scoring config, question count,
total score, maximum score, percentage, and an array of completed questions.

Each multiple-choice question includes `id`, `text`, `type: "multiple_choice"`, `status`, `score`, `max_score`, and `metadata`,
plus the following answer fields:

```json
{
  "answers": {"a": "2", "b": "4", "c": "3", "d": "5"},
  "correct": ["a", "b"],
  "selected": ["a", "c"],
  "missed": ["b"],
  "incorrect": ["c"]
}
```

`answers` contains every option, keyed by the lowercase label from that attempt's
display order. The label arrays follow that same order. `missed` lists correct options
that were not selected; `incorrect` lists selected wrong options. Empty categories
are `[]`. Rerun exports use the current attempt's order, including after reshuffling.
Version 2 replaces version 1's `correct_answers`, `selected_answers`,
`missed_correct_answers`, and `wrong_selected_answers` pairs with these fields.


## Mixed questions, marks, and hints

Choose one JSON quiz with the file picker or drop it on **Drop a JSON quiz here, or choose a file**.
The same validation applies to both paths, regardless of the file's MIME type. Multiple-file
drops are rejected. A failed upload leaves the existing quiz state intact. When replacing a completed quiz, **Back to Results** returns to the previous attempt and its exports; it is cleared only after a valid replacement loads.

Questions default to `"type": "multiple_choice"`; existing files and camelCase aliases still
work. MCQs require `correct_answers` (nonempty) and `incorrect_answers` (may be empty).
Use `"type": "short_answer"` for a multiline, unmarked response. Omit answer arrays or use
empty arrays on short answers; nonempty options are rejected. Question IDs must be unique.

```json
{
  "id": "explain-1",
  "type": "short_answer",
  "marks": 2.5,
  "question": "Explain why this works.",
  "hints": ["Consider the starting conditions.", "What stays unchanged?"],
  "metadata": {"topic": "Reasoning"}
}
```

Both types accept positive numeric `marks`, including fractions. Available marks resolve in
this order: question `marks`, top-level `marks_per_question`, `config.marks_per_question`,
then 1. Zero, negative, null, nonnumeric, and non-finite overrides are invalid. Available
marks appear on every question and result. MCQ credit and penalties scale by resolved marks.
Short answers have no awarded score. Mixed-quiz scores and percentages cover MCQs only;
all-short-answer quizzes show completion and total available marks without a grade.

Both types accept a `hints` array of nonempty plain-text strings. Each numbered button
independently reveals or hides its hint. Viewing hints does not count as answering and has
no score penalty. Once viewed, a hint stays in the attempt's history even after hiding it or
editing an answer. Omitted or empty arrays produce no hint controls or result badges.

Typed responses preserve whitespace, Unicode, and line breaks; whitespace-only answers are
unanswered. Blank submission is allowed. Reruns, settings transitions, and successful replacement
uploads clear responses and hint history. Refresh loses in-memory work; export submitted results
to keep them. There is no autosave or marking workflow for short answers.

Version 3 retains version 2's MCQ label mappings and adds `type`, `hints`, `hints_viewed`, and
`viewed_hint_indices` to every question. Indices are sorted, unique, zero-based, and refer to
source hint order. Hintless questions always export `[]`, `false`, and `[]`. Hint definitions
are exported as study material; usage records only hints actually revealed before submission.
Short-answer entries include `id`, `text`, `type: "short_answer"`, exact `response`,
`status: "answered_unmarked"` or `"unanswered_unmarked"`, `max_score`, `metadata`, and hint
fields. They omit `score` and all option fields. `max_score` always means available marks.
`question_count` covers all questions; `scored_question_count` counts MCQs. `total_available_marks`
sums all questions, while `total_score` and `total_max` cover MCQs. `percentage` is null when
there are no MCQs. Copy and download serialize the same document, including for MCQ-only quizzes.
Version 2 consumers must accommodate this versioned contract change.

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

Copy Results JSON and Download Results JSON export the same `format_version: 2` document.
It includes the quiz title, source filename, seed, scoring config, question count,
total score, maximum score, percentage, and an array of completed questions.

Each question includes `id`, `text`, `status`, `score`, `max_score`, and `metadata`,
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

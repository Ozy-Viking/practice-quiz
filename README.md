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

Copy Results JSON and Download Results JSON export the same `format_version: 4` document.
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

Choose one JSON or ZIP quiz with the file picker or drop it on **Drop a JSON or ZIP quiz here, or choose a file**.
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

Both types accept a `hints` array of nonempty plain-text strings. One “Show next hint” button
reveals hints in source order, one per activation. Revealed hints stay visible in separate
blocks with source line breaks preserved; the button disappears after the last hint. Viewing
hints does not count as answering and has no score penalty. Once viewed, a hint stays in the
attempt's history even after editing an answer. Omitted or empty arrays produce no hint controls or result badges.

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

## Images and ZIP quizzes

Both question types accept an optional `images` array in display order:

```json
"images": [
  { "src": "figures/week 1/cell.png", "alt": "Cell diagram with structure A indicated", "caption": "Figure 1" }
]
```

`src` and `alt` must be nonblank strings. `caption` is optional plain text. Omit
`images` or use `[]` for text-only questions; `null` is invalid. Images appear below
the prompt in quizzes, results, and study review. Activate an image with the mouse
or keyboard to enlarge it; Escape or **Close image** closes the dialog and restores
focus. Viewing images does not change answers, marks, or hint history.

For **standalone JSON**, use absolute HTTPS image URLs without embedded credentials.
Extensionless endpoints work. HTTP, protocol-relative, `data:`, and `blob:` sources
are rejected. Images load directly from their hosts with `referrerpolicy="no-referrer"`;
the host still receives a request. Hosts must allow public embedding. Failed remote
images show alternative text and **Retry image**. Remote bytes, format and dimensions
cannot be checked during import.

For **ZIP**, put one or more quiz JSON files at the archive root. Any root JSON
filename works (case-insensitive `.json` suffix). A sole candidate loads automatically;
multiple candidates require an explicit selection from a sorted list. `quiz.json`
has no priority. Invalid selected JSON leaves the chooser open so another file can
be selected. Canceling or returning to results preserves the previous attempt.
Nested JSON is not a quiz candidate, and unselected JSON need not be valid.

Local image references are exact, case-sensitive paths relative to the ZIP root:
`photo.png`, `figures/photo.jpg`, and `week 1/形.webp` are valid. There is no mandatory
images folder. Paths are not URL-decoded. Absolute paths, backslashes, colons,
empty segments, `.`/`..`, duplicate names, symlinks and encryption are rejected.
Ordinary directories and macOS metadata are ignored. Other unrelated files are not
decompressed, but archive-wide validation still applies. ZIPs may mix local images
and HTTPS URLs. Stored and Deflate compression are supported.

Limits are checked before accepting a replacement:

| Resource | Limit |
|---|---:|
| Uploaded ZIP | 50 MiB |
| Standalone JSON / selected manifest | 2 MiB |
| Archive entries (including directories/metadata) | 1,000 |
| Aggregate declared uncompressed bytes | 100 MiB |
| Actual imported uncompressed bytes | 100 MiB |
| Each referenced image | 10 MiB |
| Each referenced image's dimensions | 20 megapixels |

Bundled PNG, JPEG, GIF and WebP signatures and dimensions are checked before
sequential browser decoding. Corrupt or missing referenced images reject the import.
Reads stop at the byte cap plus one. No archive is extracted to disk. Shared images
use one Blob URL, retained through settings, results and reruns. Failed, canceled or
superseded imports clean up provisional URLs; successful replacement, explicit
quiz discard and app teardown release the previous assets. Refresh loses the
in-memory quiz, as before.

Try [the portable sample ZIP](examples/images/image-practice.zip), its
[source manifest](examples/images/Biology%20revision.JSON), or the
[HTTPS JSON authoring example](examples/images/url-quiz.json). The HTTPS example uses
illustrative `example.org` URLs: replace them with real public image endpoints.
Tests use generated local fixtures, never those external URLs.

Results version 4 adds each question's original `images` and top-level
`source_manifest` (the exact selected root filename, or `null` for plain JSON).
`source_file` remains the uploaded filename, including `.zip`. Copy and download
produce identical JSON, with no Blob URLs or embedded image data. **Results are an
answer record, not a self-contained image archive**: keep the source ZIP for local
images. All version 3 answer, mark, hint, and scoring fields remain unchanged.

To run browser tests against this checkout without reusing another dev server:

```sh
QUIZ_TEST_PORT=8093 npm run test:e2e -- --project=chromium --project=firefox
```

Native checks: `cargo fmt --check` and `cargo test --locked`. Browser build:
`dx build --web`. Fixtures can be regenerated with
`python3 tests/fixtures/images/generate.py` (Pillow required).

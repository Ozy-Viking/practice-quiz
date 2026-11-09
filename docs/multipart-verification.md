# Multipart questions: implementation and verification

## Implemented design

- `QuizQuestion` and `SessionQuestion` retain a top-level `group` with ordered ordinary leaves. Runtime and schema reject nested/empty groups, null parts, any `subquestions` property on leaves, and group marks/hints/options (including empty arrays and aliases). IDs are globally unique at runtime; leaf marks and their total must be finite and positive.
- `all_questions` visits context and leaves for import/image/ID validation. `session_leaves` carries the parent and positional path for responses, progress, hints, scoring, accessible names, result anchors and metadata display fallback.
- Results store only answerable leaf outcomes; the retained session supplies group boundaries and shared context. This avoids inventing an answer, status or awarded score for a group. Shared explanations appear once. Short answers remain unmarked. MCQ leaves alone contribute to the scored denominator.
- Selection, filters, counts and shuffling operate on whole top-level questions; parts retain source order. Both rerun modes clear every response and hint. Existing flat ChaCha8 and True/False ordering remain covered by the original seeded fixtures.
- Flat selected sessions export unchanged v4 fields. Sessions containing groups export v5 nested `subquestions`, top-level `question_count`, leaf `answerable_question_count` and MCQ-leaf `scored_question_count`. Export validation checks IDs, order, counts, kinds and retained source group boundaries. Metadata and image references remain separate and original.
- Shared context and all subquestions now sit inside one continuous card in both the quiz and results, with thin dividers between parts. Each part keeps its independent response, images, hints and marks.
- All prompt images stack individually on the centerline and fit their container. Enlarged images are centered too. Shared text, prompt text and explanations preserve line breaks. Textarea accessible names include the positional label, such as `Q1(a)`, and the part prompt.
- The schema retains `definitions.QuizQuestion.properties` for upload help and whitespace tests. README and upload help describe group behavior.

## Example

Upload [`authentication-quiz.zip`](../examples/authentication/authentication-quiz.zip). It contains one six-mark question with **four independent short-answer textareas**, a shared Alice/Bob protocol diagram, and a diagram and hint for every part. The JSON, PNGs and reproducible Pillow generator are under `examples/authentication/`. Local images require the ZIP.

## Checks executed

| Check | Result |
| --- | --- |
| `cargo fmt --check` | Passed |
| `cargo test --locked` | 75 passed |
| `cargo clippy --all-targets --locked -- -D warnings` | Passed |
| `cargo check --target wasm32-unknown-unknown --locked` | Passed |
| `dx build --web` | Passed; CSS and schema assets bundled |
| `python -m unittest discover -s tests -p schema_test.py` | 4 passed |
| TypeScript check of `tests/multipart.spec.ts` with ES2022, Node16 and Node types | Passed |
| `npx playwright test --project=chromium` | Final full suite: 81 passed |
| Full Firefox suite before final image stacking case | 80 passed |
| Final multipart suite, Chromium and Firefox | 18 passed (9 each) |
| WebKit full-suite attempt | 80 launch failures: host lacks required ICU/XML/Flite libraries; browser behavior unverified |
| `git diff --check` | Passed |

The new tests cover invalid group shapes, IDs and marks; shared/child image import and provenance; four independent responses with exact whitespace and partial completion; leaf hints; parent filters; selection/shuffle order; mixed scoring; Study Review anchors and option lookup; nested copy/download equality; rerun clearing; and image sizing/centering at 1440 × 1000 and 390 × 844 viewports. The unchanged flat seeded fixture runs in native and browser tests.

## Real screenshots

Captured with native Python Playwright against the built Dioxus app, then opened and visually inspected. All five bundled images decoded, and mobile pages had no horizontal overflow.

| Screenshot | Dimensions |
| --- | --- |
| [Desktop quiz](screenshots/multipart-desktop-quiz.png) | 1440 × 4212 |
| [Desktop results](screenshots/multipart-desktop-results.png) | 1440 × 4346 |
| [Mobile quiz](screenshots/multipart-mobile-quiz.png) | 390 × 3818 |
| [Mobile results](screenshots/multipart-mobile-results.png) | 390 × 4947 |

The capture script is [`screenshots/capture_multipart.py`](screenshots/capture_multipart.py). With a local server at port 8082, run `uv run --with playwright==1.60.0 python docs/screenshots/capture_multipart.py`. Desktop and mobile captures use light mode and filled, independent responses; results show the shared explanation once and separate unmarked responses.

No commit or pull request was created.

## Single-card refinement

`cargo check --locked`, `cargo fmt --check`, `git diff --check`, and all 30 Chromium multipart/short-answer browser tests passed after placing context and parts inside one card. The four screenshots were recaptured against the rebuilt app.

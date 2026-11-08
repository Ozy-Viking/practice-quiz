# Practice Quiz

[Open Practice Quiz](https://ozy-viking.github.io/practice-quiz/) and upload a JSON or ZIP quiz.

## Questions JSON

### Quiz fields

| Field | Required? | Meaning / default |
|---|---|---|
| `$schema` | No | Editor validation: `https://ozy-viking.github.io/practice-quiz/quiz-schema.json`. |
| `title` | No | Quiz heading; falls back to `config.title`, then `Practice Quiz`. |
| `questions` | Yes | Nonempty array of questions. |
| `config` | No | Default settings; individual fields are optional. |
| `config.title` | No | Fallback quiz title. |
| `config.description` | No | Quiz description. |
| `config.default_question_count` | No | Initial question count; positive integer, capped at the number available. Defaults to all questions. |
| `config.marks_per_question` | No | Default available marks; positive number, including fractions. Defaults to `1`. |
| `marks_per_question` | No | Overrides `config.marks_per_question`. |
| `config.allow_negative_mark` | No | Allows scores below zero; defaults to `false`. Alias: `config.negative_marks`. |
| `negative_marks` | No | Overrides `config.allow_negative_mark`. |

### Question fields

| Field | Required? | Meaning / default |
|---|---|---|
| `id` | Yes | String unique within the quiz. |
| `question` | Yes | Prompt text. |
| `type` | No | `multiple_choice` (default) or `short_answer` (unmarked text response). |
| `correct_answers` | MCQ only | Nonempty array of correct strings. Alias: `correctAnswers`. For short answers, omit or use `[]`. |
| `incorrect_answers` | MCQ only | Array of wrong strings; may be empty. Alias: `incorrectAnswers`. For short answers, omit or use `[]`. |
| `marks` | No | Positive available marks, including fractions; overrides the quiz default. |
| `hints` | No | Nonblank plain-text strings, revealed one at a time in array order without a score penalty. Defaults to `[]`. |
| `images` | No | Image objects in display order; defaults to `[]`. |
| `explanation` | No | Post-submission explanation; used when `metadata.explanation` is absent. |
| `metadata` | No | Study information below; all fields optional. Alias: `hiddenInfo`. |

Use either a field's canonical name or its alias, never both.

| Metadata field | Required? | Meaning |
|---|---|---|
| `study_location` | No | Teaching location, such as `Week 1`; enables location filtering. |
| `topic` | No | Subject area; enables topic filtering. |
| `timestamp` | No | Lecture timestamp or source reference. |
| `answer` | No | Original answer-key reference. |
| `explanation` | No | Post-submission explanation; takes priority over question-level `explanation`. |
| `notes` | No | Study notes, shown when no explanation exists. |

### Images and uploads

| Image field | Required? | Meaning |
|---|---|---|
| `src` | Yes | Absolute HTTPS URL without credentials, or an exact, case-sensitive path relative to the detected package root: the ZIP root or its one enclosing quiz folder. Replace illustrative `example.org` URLs with public image URLs. |
| `alt` | Yes | Nonblank alternative text describing the image. |
| `caption` | No | Plain-text caption below the image. |

| Upload | Requirements |
|---|---|
| JSON | Image sources must be HTTPS URLs; hosts must allow embedding. |
| ZIP | Quiz JSON files go at the archive root or directly inside one enclosing folder; choose one if several exist at the detected package root. May mix HTTPS URLs and bundled PNG, JPEG, GIF or WebP images. Local paths cannot use absolute paths, backslashes, colons, empty segments or `.` / `..`. |
| Limits | JSON: 2 MiB. ZIP: 50 MiB, 1,000 entries, 100 MiB total uncompressed. Each bundled image: 10 MiB and 20 megapixels; expanded PNG metadata: 8 MiB per image. |

### ZIP structure

A ZIP bundles the quiz JSON and its local images into one upload. You can compress the folder's contents, putting JSON directly at the archive root:

```text
biology.zip
├── quiz.json
├── chloroplast.png
├── quiz/
│   └── leaf.jpg
└── images/
    └── cells.png
```

In this layout, `quiz.json` uses `"src": "chloroplast.png"`, `"src": "quiz/leaf.jpg"`, or `"src": "images/cells.png"`. Paths are case-sensitive and relative to the ZIP root. Folder names are your choice; neither `quiz` nor `images` is required.

You can also compress the quiz folder itself, keeping one enclosing folder:

```text
quiz.zip
└── quiz/
    ├── quiz.json
    └── images/
        └── a/
            └── b/
                └── c/
                    └── d/
                        └── image.png
```

Here the package root is `quiz/`, so `quiz.json` uses `"src": "images/a/b/c/d/image.png"`. Do not include the enclosing `quiz/` prefix in `src`. Image subfolders can be nested arbitrarily; the one-folder limit applies only to quiz JSON discovery. `outer/quiz/quiz.json` is too deep.

Try the [wrapped sample ZIP](examples/images/wrapped-image-practice.zip) for a complete package with both question types and bundled images.

If archive-root JSON files exist, they are the only quiz candidates. The app does not fall back to an enclosing folder when a root JSON file is invalid. Otherwise, all non-metadata files must share one enclosing folder, with quiz JSON directly inside it. Substantive files at the archive root or in sibling folders make that enclosing-folder layout unsupported. Empty directory records and recognized macOS metadata (`.DS_Store` and `__MACOSX/` at the archive root or within the enclosing folder) do not affect layout detection; archive validation and limits still apply to every entry.

A single JSON candidate loads automatically. Multiple candidates appear in a sorted “Choose a quiz” list and require an explicit selection; `quiz.json` has no special priority. Only the selected JSON is parsed and validated. If it is invalid, the error is shown and you can select another candidate. JSON files deeper than the detected package root are ignored for quiz selection. All selected quizzes resolve local image paths from that same package root and can share bundled images.

### Three-question example

Save as `quiz.json` at the root of `biology.zip`, with `cells.png` in its `images` folder as shown above.

```json
{
  "$schema": "https://ozy-viking.github.io/practice-quiz/quiz-schema.json",
  "title": "Biology revision",
  "config": {
    "title": "Biology practice",
    "description": "Cells and photosynthesis",
    "default_question_count": 3,
    "marks_per_question": 1,
    "allow_negative_mark": false
  },
  "marks_per_question": 2,
  "negative_marks": false,
  "questions": [
    {
      "id": "q1",
      "type": "multiple_choice",
      "question": "Which organelle carries out photosynthesis?",
      "marks": 2,
      "correct_answers": ["Chloroplast"],
      "incorrect_answers": ["Nucleus", "Mitochondrion"],
      "hints": ["Look for the organelle containing chlorophyll."],
      "images": [
        {
          "src": "https://example.org/chloroplast.png",
          "alt": "A chloroplast with stacks of internal membranes",
          "caption": "Figure 1: Chloroplast"
        }
      ],
      "explanation": "Chloroplasts contain chlorophyll for photosynthesis.",
      "metadata": {
        "study_location": "Week 1",
        "topic": "Photosynthesis",
        "timestamp": "00:12:30",
        "answer": "Worksheet 1, Q1",
        "explanation": "Chloroplasts contain chlorophyll for photosynthesis.",
        "notes": "Review the chloroplast diagram."
      }
    },
    {
      "id": "q2",
      "type": "multiple_choice",
      "question": "Which structures occur in both plant and animal cells? Select all that apply.",
      "marks": 2,
      "correct_answers": ["Cell membrane", "Cytoplasm"],
      "incorrect_answers": ["Cell wall", "Chloroplast"],
      "hints": ["Compare the two cell diagrams.", "Focus on structures shared by both cells."],
      "images": [
        {
          "src": "images/cells.png",
          "alt": "Plant and animal cell diagrams shown side by side",
          "caption": "Figure 2: Comparing cells"
        }
      ],
      "metadata": {
        "topic": "Cells",
        "explanation": "Both have a cell membrane and cytoplasm; animal cells lack cell walls and chloroplasts."
      }
    },
    {
      "id": "q3",
      "type": "short_answer",
      "question": "Explain how photosynthesis supports a plant.",
      "marks": 2.5,
      "hints": [],
      "metadata": {
        "topic": "Photosynthesis",
        "notes": "Include light energy, carbon dioxide and water."
      }
    }
  ]
}
```

## Results JSON

**Copy Results JSON** and **Download Results JSON** export the same completed attempt.

| Quiz field | Meaning |
|---|---|
| `quiz_title` | Resolved quiz title. |
| `source_file` | Uploaded filename. |
| `source_manifest` | Exact selected JSON path within the ZIP, such as `quiz.json` or `quiz/quiz.json`; `null` for standalone JSON. |
| `seed` | Numeric draw seed (`0`–`4294967295`). Matching source order, filters, count and seed reproduce the draw. Reruns may reshuffle the same questions while retaining the original seed. |
| `config` | Effective quiz settings after top-level overrides. |
| `question_count` / `scored_question_count` | All questions / MCQs only. |
| `total_available_marks` | Available marks across all questions, including short answers. |
| `total_score` / `total_max` | Awarded / available MCQ marks only. |
| `percentage` | `total_score / total_max × 100`; `null` with no MCQs. |
| `questions` | Results in the current attempt's display order. |

| Question field | Present for | Meaning |
|---|---|---|
| `id`, `type` | All | Source identifiers. |
| `text` | All | Source `question` text. |
| `images` | All | Original references and descriptions; `[]` for no images. Keep the source ZIP to retain bundled images. |
| `metadata` | All | Study information with the resolved explanation; absent values become `null`. |
| `max_score` | All | Available marks. |
| `hints` | All | All source hints, including unrevealed hints. |
| `hints_viewed` | All | Whether any hint was revealed. |
| `viewed_hint_indices` | All | Sorted, unique, zero-based indices of revealed hints; otherwise `[]`. |
| `status` | MCQ | `correct`, `partially_correct`, `incorrect` or `unanswered`. Partial correctness can still score zero after penalties. |
| `status` | Short answer | `answered_unmarked` or `unanswered_unmarked`; blank or whitespace-only responses are unanswered. |
| `score` | MCQ | `max_score × (correct selected / correct options − wrong selected / wrong options)`, floored at zero unless negative marks are enabled. Unanswered questions score zero; with no wrong options, the penalty is zero. |
| `answers` | MCQ | Every option keyed by its lowercase label in this attempt's display order. |
| `correct` / `selected` | MCQ | Correct / selected labels, in display order. |
| `missed` / `incorrect` | MCQ | Correct labels not selected / wrong labels selected; empty categories are `[]`. |
| `response` | Short answer | Exact typed text, preserving whitespace and line breaks; no `score` or option fields. |

### Matching results example

```json
{
  "format_version": 4,
  "quiz_title": "Biology revision",
  "source_file": "biology.zip",
  "source_manifest": "quiz.json",
  "seed": 0,
  "config": {
    "marks_per_question": 2.0,
    "allow_negative_mark": false,
    "title": "Biology practice",
    "description": "Cells and photosynthesis",
    "default_question_count": 3
  },
  "question_count": 3,
  "total_score": 2.0,
  "total_max": 4.0,
  "percentage": 50.0,
  "scored_question_count": 2,
  "total_available_marks": 6.5,
  "questions": [
    {
      "images": [
        {
          "src": "https://example.org/chloroplast.png",
          "alt": "A chloroplast with stacks of internal membranes",
          "caption": "Figure 1: Chloroplast"
        }
      ],
      "id": "q1",
      "text": "Which organelle carries out photosynthesis?",
      "status": "correct",
      "max_score": 2.0,
      "metadata": {
        "study_location": "Week 1",
        "timestamp": "00:12:30",
        "topic": "Photosynthesis",
        "answer": "Worksheet 1, Q1",
        "explanation": "Chloroplasts contain chlorophyll for photosynthesis.",
        "notes": "Review the chloroplast diagram."
      },
      "hints": ["Look for the organelle containing chlorophyll."],
      "hints_viewed": false,
      "viewed_hint_indices": [],
      "type": "multiple_choice",
      "score": 2.0,
      "answers": {
        "a": "Chloroplast",
        "b": "Mitochondrion",
        "c": "Nucleus"
      },
      "correct": ["a"],
      "selected": ["a"],
      "missed": [],
      "incorrect": []
    },
    {
      "images": [
        {
          "src": "images/cells.png",
          "alt": "Plant and animal cell diagrams shown side by side",
          "caption": "Figure 2: Comparing cells"
        }
      ],
      "id": "q2",
      "text": "Which structures occur in both plant and animal cells? Select all that apply.",
      "status": "partially_correct",
      "max_score": 2.0,
      "metadata": {
        "study_location": null,
        "timestamp": null,
        "topic": "Cells",
        "answer": null,
        "explanation": "Both have a cell membrane and cytoplasm; animal cells lack cell walls and chloroplasts.",
        "notes": null
      },
      "hints": ["Compare the two cell diagrams.", "Focus on structures shared by both cells."],
      "hints_viewed": true,
      "viewed_hint_indices": [0],
      "type": "multiple_choice",
      "score": 0.0,
      "answers": {
        "a": "Cytoplasm",
        "b": "Chloroplast",
        "c": "Cell membrane",
        "d": "Cell wall"
      },
      "correct": ["a", "c"],
      "selected": ["c", "d"],
      "missed": ["a"],
      "incorrect": ["d"]
    },
    {
      "images": [],
      "id": "q3",
      "text": "Explain how photosynthesis supports a plant.",
      "status": "answered_unmarked",
      "max_score": 2.5,
      "metadata": {
        "study_location": null,
        "timestamp": null,
        "topic": "Photosynthesis",
        "answer": null,
        "explanation": null,
        "notes": "Include light energy, carbon dioxide and water."
      },
      "hints": [],
      "hints_viewed": false,
      "viewed_hint_indices": [],
      "type": "short_answer",
      "response": "Photosynthesis uses light energy to convert carbon dioxide and water into sugars.\nThese sugars support growth and respiration."
    }
  ]
}
```

use dioxus::prelude::*;

use crate::model::{AppPhase, QuizFile, initial_question_count, validate_quiz, EXAMPLE_QUIZ_JSON};

// Ensure the schema file is included in the build without a hash suffix.
#[used]
static SCHEMA_ASSET: Asset = asset!(
    "/assets/quiz-schema.json",
    AssetOptions::builder().with_hash_suffix(false)
);

fn schema_url() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.location().href().ok())
            .map(|href| {
                let base = href.trim_end_matches('/');
                format!("{base}/assets/quiz-schema.json")
            })
            .unwrap_or_else(|| "/assets/quiz-schema.json".to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "/assets/quiz-schema.json".to_string()
    }
}

fn copy_to_clipboard(text: String, mut copied: Signal<bool>) {
    spawn(async move {
        #[cfg(target_arch = "wasm32")]
        if let Some(clipboard) = web_sys::window().map(|w| w.navigator().clipboard()) {
            let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&text)).await;
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = text;
        copied.set(true);
    });
}

#[component]
pub fn UploadView(
    mut phase: Signal<AppPhase>,
    mut quiz: Signal<Option<QuizFile>>,
    mut question_count: Signal<usize>,
    mut file_name: Signal<String>,
    mut load_error: Signal<String>,
) -> Element {
    let mut show_format = use_signal(|| false);
    let copied = use_signal(|| false);
    let url = schema_url();
    let example_json = EXAMPLE_QUIZ_JSON.replace("SCHEMA_URL", &url);

    rsx! {
        div { class: "card upload-card",
            h2 { "Load Quiz File" }
            p { class: "hint", "Upload a JSON file containing your quiz questions." }
            input {
                r#type: "file",
                accept: ".json,application/json",
                onchange: move |evt: FormEvent| {
                    let files = evt.files();
                    let file = match files.into_iter().next() {
                        Some(f) => f,
                        None => return,
                    };
                    let name = file.name();
                    spawn(async move {
                        let text = match file.read_string().await {
                            Ok(t) => t,
                            Err(e) => {
                                load_error.set(format!("Failed to read file: {e}"));
                                return;
                            }
                        };
                        let parsed: Result<QuizFile, _> = serde_json::from_str(&text);
                        match parsed {
                            Ok(q) => match validate_quiz(&q) {
                                Ok(()) => {
                                    question_count.set(initial_question_count(&q));
                                    file_name.set(name);
                                    quiz.set(Some(q));
                                    phase.set(AppPhase::Configuring);
                                    load_error.set(String::new());
                                }
                                Err(msg) => load_error.set(msg),
                            },
                            Err(e) => load_error.set(format!("Invalid JSON: {e}")),
                        }
                    });
                },
            }
            if !load_error.read().is_empty() {
                div { class: "error-banner", {load_error.read().clone()} }
            }

            div { class: "format-reference",
                button {
                    class: "format-toggle",
                    r#type: "button",
                    onclick: move |_| {
                        let v = *show_format.read();
                        show_format.set(!v);
                    },
                    span { class: "format-toggle-icon", if *show_format.read() { "▾" } else { "▸" } }
                    "JSON Format Reference"
                }
                if *show_format.read() {
                    div { class: "format-body",
                        div { class: "schema-url-row",
                            span { class: "schema-url-label", "JSON Schema" }
                            a {
                                class: "schema-url",
                                href: "{url}",
                                download: "quiz-schema.json",
                                target: "_blank",
                                "{url}"
                            }
                            span { class: "schema-url-hint",
                                "— use this URL directly as your "
                                code { "\"$schema\"" }
                                " value for IDE validation"
                            }
                        }

                        div { class: "example-wrapper",
                            button {
                                class: if *copied.read() { "copy-btn copied" } else { "copy-btn" },
                                r#type: "button",
                                onclick: {
                                    let text = example_json.clone();
                                    move |_| copy_to_clipboard(text.clone(), copied)
                                },
                                if *copied.read() { "✓ Copied" } else { "Copy" }
                            }
                            pre { class: "format-example", "{example_json}" }
                        }

                        div { class: "field-ref",
                            div { class: "field-ref-section",
                                div { class: "field-ref-heading", "Top-level" }
                                div { class: "field-ref-row",
                                    code { "title" }
                                    span { class: "field-ref-type", "string" }
                                    span { "Display name shown in the quiz header." }
                                }
                                div { class: "field-ref-row",
                                    code { "questions" }
                                    span { class: "field-ref-type field-ref-required", "array  required" }
                                    span { "List of question objects (see below)." }
                                }
                                div { class: "field-ref-row",
                                    code { "config" }
                                    span { class: "field-ref-type", "object" }
                                    span { "Scoring and display settings (see below)." }
                                }
                                div { class: "field-ref-row",
                                    code { "marks_per_question" }
                                    span { class: "field-ref-type", "number" }
                                    span { "Shorthand override for config.marks_per_question." }
                                }
                                div { class: "field-ref-row",
                                    code { "negative_marks" }
                                    span { class: "field-ref-type", "boolean" }
                                    span { "Shorthand override for config.allow_negative_mark." }
                                }
                            }
                            div { class: "field-ref-section",
                                div { class: "field-ref-heading", "config" }
                                div { class: "field-ref-row",
                                    code { "marks_per_question" }
                                    span { class: "field-ref-type", "number" }
                                    span { "Points awarded for a fully correct answer. Default: 1.0." }
                                }
                                div { class: "field-ref-row",
                                    code { "allow_negative_mark" }
                                    span { class: "field-ref-type", "boolean" }
                                    span { "Allow scores below zero for wrong selections. Default: false." }
                                }
                                div { class: "field-ref-row",
                                    code { "default_question_count" }
                                    span { class: "field-ref-type", "integer" }
                                    span { "Pre-fills the question count input on the config screen." }
                                }
                                div { class: "field-ref-row",
                                    code { "description" }
                                    span { class: "field-ref-type", "string" }
                                    span { "Optional description displayed on the config screen." }
                                }
                            }
                            div { class: "field-ref-section",
                                div { class: "field-ref-heading", "questions[ ]" }
                                div { class: "field-ref-row",
                                    code { "id" }
                                    span { class: "field-ref-type field-ref-required", "string  required" }
                                    span { "Unique identifier for this question." }
                                }
                                div { class: "field-ref-row",
                                    code { "question" }
                                    span { class: "field-ref-type field-ref-required", "string  required" }
                                    span { "Question text displayed to the user." }
                                }
                                div { class: "field-ref-row",
                                    code { "correct_answers" }
                                    span { class: "field-ref-type field-ref-required", "string[]  required" }
                                    span {
                                        "One or more correct answer strings. Alias: "
                                        code { "correctAnswers" }
                                        "."
                                    }
                                }
                                div { class: "field-ref-row",
                                    code { "incorrect_answers" }
                                    span { class: "field-ref-type field-ref-required", "string[]  required" }
                                    span {
                                        "Wrong answer strings. Alias: "
                                        code { "incorrectAnswers" }
                                        ". Total answers ≤ 5."
                                    }
                                }
                                div { class: "field-ref-row",
                                    code { "explanation" }
                                    span { class: "field-ref-type", "string" }
                                    span { "Explanation shown after submission. Shorthand for metadata.explanation." }
                                }
                                div { class: "field-ref-row",
                                    code { "metadata" }
                                    span { class: "field-ref-type", "object" }
                                    span {
                                        "Study metadata. Alias: "
                                        code { "hiddenInfo" }
                                        " (see below)."
                                    }
                                }
                            }
                            div { class: "field-ref-section",
                                div { class: "field-ref-heading", "metadata" }
                                div { class: "field-ref-row",
                                    code { "topic" }
                                    span { class: "field-ref-type", "string" }
                                    span { "Subject area. Used by the Topic filter on the config screen." }
                                }
                                div { class: "field-ref-row",
                                    code { "study_location" }
                                    span { class: "field-ref-type", "string" }
                                    span { "Where this was taught (e.g. \"Week 3, slide 7\"). Used by the Location filter." }
                                }
                                div { class: "field-ref-row",
                                    code { "explanation" }
                                    span { class: "field-ref-type", "string" }
                                    span { "Explanation shown in the results study-info panel." }
                                }
                                div { class: "field-ref-row",
                                    code { "notes" }
                                    span { class: "field-ref-type", "string" }
                                    span { "Additional study notes. Shown if no explanation is set." }
                                }
                                div { class: "field-ref-row",
                                    code { "answer" }
                                    span { class: "field-ref-type", "string" }
                                    span { "Original answer key reference. Shown in the results study-info panel." }
                                }
                                div { class: "field-ref-row",
                                    code { "timestamp" }
                                    span { class: "field-ref-type", "string" }
                                    span { "Timestamp or slide reference. Shown in the results study-info panel." }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

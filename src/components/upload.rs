use dioxus::prelude::*;

use crate::model::{AppPhase, EXAMPLE_QUIZ_JSON, QuizFile, initial_question_count, validate_quiz};

const SCHEMA_JSON: &str = include_str!("../../assets/quiz-schema.json");

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
            .and_then(|href| schema_url_from_href(&href).ok())
            .unwrap_or_else(|| "/assets/quiz-schema.json".to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "/assets/quiz-schema.json".to_string()
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn schema_url_from_href(href: &str) -> Result<String, url::ParseError> {
    let mut base = url::Url::parse(href)?;
    base.set_query(None);
    base.set_fragment(None);

    if !base.path().ends_with('/') {
        let path = format!("{}/", base.path());
        base.set_path(&path);
    }

    base.join("assets/quiz-schema.json")
        .map(|schema| schema.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_url_uses_current_url_without_query_or_fragment() {
        assert_eq!(
            schema_url_from_href("http://127.0.0.1:44947/practice-quiz/?fresh=firefox3#top")
                .unwrap(),
            "http://127.0.0.1:44947/practice-quiz/assets/quiz-schema.json"
        );
        assert_eq!(
            schema_url_from_href("http://127.0.0.1:44947/").unwrap(),
            "http://127.0.0.1:44947/assets/quiz-schema.json"
        );
    }
}

fn copy_to_clipboard(text: String, mut copied: Signal<bool>) {
    spawn(async move {
        copied.set(false);
        #[cfg(target_arch = "wasm32")]
        if let Some(clipboard) = web_sys::window().map(|w| w.navigator().clipboard()) {
            let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&text)).await;
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = text;
        copied.set(true);
    });
}

struct FieldRow {
    name: String,
    type_label: String,
    required: bool,
    description: String,
}

fn prop_type_label(prop: &serde_json::Value) -> String {
    if prop.get("$ref").is_some() {
        return "object".to_string();
    }
    match prop.get("type").and_then(|t| t.as_str()) {
        Some("array") => {
            let item_type = prop["items"]
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("object");
            format!("{item_type}[]")
        }
        Some(t) => t.to_string(),
        None => String::new(),
    }
}

/// Extract field rows from a properties object inside the schema.
/// `required_names` marks fields as required regardless of the schema `required` array.
/// Fields whose description begins with "Alias for" are skipped (they're camelCase aliases).
fn fields_from_props(props: &serde_json::Value, required_names: &[&str]) -> Vec<FieldRow> {
    let req: std::collections::HashSet<&str> = required_names.iter().cloned().collect();
    let Some(map) = props.as_object() else {
        return vec![];
    };
    map.iter()
        .filter(|(name, prop)| {
            // Drop the bare $schema entry and any camelCase alias fields
            *name != "$schema"
                && prop["description"]
                    .as_str()
                    .map_or(true, |d| !d.starts_with("Alias for"))
        })
        .map(|(name, prop)| FieldRow {
            name: name.clone(),
            type_label: prop_type_label(prop),
            required: req.contains(name.as_str()),
            description: prop["description"].as_str().unwrap_or("").to_string(),
        })
        .collect()
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
    let example_copied = use_signal(|| false);
    let schema_copied = use_signal(|| false);
    let url = schema_url();
    let example_json = EXAMPLE_QUIZ_JSON.replace("SCHEMA_URL", &url);

    // Parse schema once; used only when the format reference panel is open.
    let schema: serde_json::Value =
        serde_json::from_str(SCHEMA_JSON).unwrap_or(serde_json::Value::Null);

    let top_fields = fields_from_props(&schema["properties"], &["questions"]);
    let config_fields = fields_from_props(&schema["definitions"]["QuizConfig"]["properties"], &[]);
    // correct_answers + incorrect_answers are required via oneOf in the schema
    let question_fields = fields_from_props(
        &schema["definitions"]["QuizQuestion"]["properties"],
        &["id", "question", "correct_answers", "incorrect_answers"],
    );
    let metadata_fields = fields_from_props(
        &schema["definitions"]["QuestionMetadata"]["properties"],
        &[],
    );

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
                            Ok(q) => {
                                match validate_quiz(&q) {
                                    Ok(()) => {
                                        question_count.set(initial_question_count(&q));
                                        file_name.set(name);
                                        quiz.set(Some(q));
                                        phase.set(AppPhase::Configuring);
                                        load_error.set(String::new());
                                    }
                                    Err(msg) => load_error.set(msg),
                                }
                            }
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
                    span { class: "format-toggle-icon",
                        if *show_format.read() {
                            "▾"
                        } else {
                            "▸"
                        }
                    }
                    "JSON Format Reference"
                }
                if *show_format.read() {
                    div { class: "format-body",
                        div { class: "schema-url-row",
                            span { class: "schema-url-label", "JSON Schema" }
                            button {
                                class: if *schema_copied.read() { "schema-url schema-url-button copied" } else { "schema-url schema-url-button" },
                                r#type: "button",
                                onclick: {
                                    let text = url.clone();
                                    move |_| copy_to_clipboard(text.clone(), schema_copied)
                                },
                                span { class: "schema-url-text", "{url}" }
                                if *schema_copied.read() {
                                    span { class: "schema-url-copied", "✓" }
                                }
                            }
                            span { class: "schema-url-hint",
                                "— click to copy this URL for your "
                                code { "\"$schema\"" }
                                " value"
                            }
                        }

                        div { class: "example-wrapper",
                            button {
                                class: if *example_copied.read() { "copy-btn copied" } else { "copy-btn" },
                                r#type: "button",
                                onclick: {
                                    let text = example_json.clone();
                                    move |_| copy_to_clipboard(text.clone(), example_copied)
                                },
                                if *example_copied.read() {
                                    "✓ Copied"
                                } else {
                                    "Copy"
                                }
                            }
                            pre { class: "format-example", "{example_json}" }
                        }

                        div { class: "field-ref",
                            {field_ref_section("Top-level", &top_fields)}
                            {field_ref_section("config", &config_fields)}
                            {field_ref_section("questions[ ]", &question_fields)}
                            {field_ref_section("metadata", &metadata_fields)}
                        }
                    }
                }
            }
        }
    }
}

fn field_ref_section(heading: &str, rows: &[FieldRow]) -> Element {
    rsx! {
        div { class: "field-ref-section",
            div { class: "field-ref-heading", "{heading}" }
            for row in rows {
                div { class: "field-ref-row",
                    code { "{row.name}" }
                    span { class: if row.required { "field-ref-type field-ref-required" } else { "field-ref-type" },
                        if row.required {
                            "{row.type_label}  required"
                        } else {
                            "{row.type_label}"
                        }
                    }
                    if !row.description.is_empty() {
                        span { "{row.description}" }
                    }
                }
            }
        }
    }
}

use crate::import::{self, AssetStore, ImportSource};
use crate::model::*;
use dioxus::html::{FileData, HasFileData};
use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

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
            schema_url_from_href("http://127.0.0.1:44947/practice-quiz/?fresh=firefox3#top",)
                .unwrap(),
            "http://127.0.0.1:44947/practice-quiz/assets/quiz-schema.json",
        );
        assert_eq!(
            schema_url_from_href("http://127.0.0.1:44947/").unwrap(),
            "http://127.0.0.1:44947/assets/quiz-schema.json",
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

async fn read_quiz_file(file: &FileData) -> Result<Vec<u8>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use dioxus::web::WebFileExt;
        let web_file = file.get_web_file().ok_or("Browser file unavailable")?;
        let limit = if file.name().to_ascii_lowercase().ends_with(".zip") {
            import::ZIP_LIMIT
        } else {
            import::JSON_LIMIT
        };
        if web_file.size() > limit as f64 {
            return Err(format!("Upload exceeds {} MiB", limit / 1024 / 1024));
        }
        let buffer = wasm_bindgen_futures::JsFuture::from(web_file.array_buffer())
            .await
            .map_err(|e| format!("{e:?}"))?;
        Ok(js_sys::Uint8Array::new(&buffer).to_vec())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        file.read_bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| e.to_string())
    }
}

fn cancel_pending(mut pending_task: Signal<Option<dioxus::core::Task>>) {
    let task = pending_task.write().take();
    if let Some(task) = task {
        task.cancel();
    }
}

fn is_file_drag(event: &DragEvent) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        event
            .downcast::<web_sys::DragEvent>()
            .and_then(|event| event.data_transfer())
            .is_some_and(|data| {
                data.types()
                    .iter()
                    .any(|kind| kind.as_string().as_deref() == Some("Files"))
            })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        !event.files().is_empty()
    }
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
                    .is_none_or(|d| !d.starts_with("Alias for"))
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
    mut session: Signal<Vec<SessionQuestion>>,
    mut selections: Signal<Responses>,
    mut hint_usage: Signal<HintUsage>,
    mut results: Signal<Option<ResultsData>>,
    mut question_pool: Signal<Vec<QuizQuestion>>,
    mut selected_topics: Signal<HashSet<String>>,
    mut selected_locations: Signal<HashSet<String>>,
    mut topic_regex_input: Signal<String>,
    mut location_regex_input: Signal<String>,
    mut seed_input: Signal<String>,
    mut active_seed: Signal<u32>,
) -> Element {
    let mut assets = use_context::<Signal<Rc<AssetStore>>>();
    let mut pending_task = use_signal(|| None::<dioxus::core::Task>);
    let mut pending = use_signal(|| None::<(String, ImportSource, u64)>);
    let mut loading = use_signal(|| false);
    let mut dragging = use_signal(|| false);
    let mut drag_depth = use_signal(|| 0usize);
    let mut upload_generation = use_signal(|| 0u64);
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
    // Option arrays are required only for multiple-choice questions.
    let question_fields = fields_from_props(
        &schema["definitions"]["QuizQuestion"]["properties"],
        &["id", "question"],
    );
    let image_fields = fields_from_props(
        &schema["definitions"]["QuizImage"]["properties"],
        &["src", "alt"],
    );
    let metadata_fields = fields_from_props(
        &schema["definitions"]["QuestionMetadata"]["properties"],
        &[],
    );
    let mut finish =
        move |name: String, source: ImportSource, manifest: Option<String>, generation: u64| {
            loading.set(true);
            load_error.set(String::new());
            let task = spawn(async move {
                let imported =
                    import::load(source, manifest, || generation == *upload_generation.peek())
                        .await;
                if generation != *upload_generation.peek() {
                    return;
                }
                pending_task.set(None);
                loading.set(false);
                match imported {
                    Ok((q, new_assets)) => {
                        session.set(Vec::new());
                        selections.set(HashMap::new());
                        hint_usage.set(HashMap::new());
                        results.set(None);
                        question_pool.set(Vec::new());
                        selected_topics.set(available_topics(&q).into_iter().collect());
                        selected_locations.set(available_locations(&q).into_iter().collect());
                        topic_regex_input.set(String::new());
                        location_regex_input.set(String::new());
                        seed_input.set(rand::random::<u32>().to_string());
                        active_seed.set(0);
                        question_count.set(initial_question_count(&q));
                        file_name.set(name);
                        assets.set(new_assets);
                        pending.set(None);
                        quiz.set(Some(q));
                        phase.set(AppPhase::Configuring);
                        load_error.set(String::new());
                    }
                    Err(error) => load_error.set(error),
                }
            });
            pending_task.set(Some(task));
        };
    let mut upload = move |files: Vec<FileData>| {
        if files.is_empty() {
            return;
        }
        cancel_pending(pending_task);
        dragging.set(false);
        drag_depth.set(0);
        let generation = *upload_generation.peek() + 1;
        upload_generation.set(generation);
        pending.set(None);
        loading.set(false);
        load_error.set(String::new());
        if files.len() != 1 {
            load_error.set("Choose or drop exactly one quiz file at a time.".into());
            return;
        }
        let file = files.into_iter().next().unwrap();
        let name = file.name();
        loading.set(true);
        let task = spawn(async move {
            let read = read_quiz_file(&file).await;
            if generation != *upload_generation.peek() {
                return;
            }
            pending_task.set(None);
            loading.set(false);
            match read
                .map_err(|e| format!("Failed to read file: {e}"))
                .and_then(|bytes| import::prepare(&name, bytes))
            {
                Ok(source) => {
                    let manifest = if let ImportSource::Archive(archive) = &source {
                        if archive.candidates.len() > 1 {
                            pending.set(Some((name, source, generation)));
                            return;
                        }
                        Some(archive.candidates[0].clone())
                    } else {
                        None
                    };
                    finish(name, source, manifest, generation);
                }
                Err(error) => load_error.set(error),
            }
        });
        pending_task.set(Some(task));
    };
    rsx! {
        div { class: "card upload-card",
            h2 { "Load Quiz File" }
            p { class: "hint", "Upload a JSON or ZIP file containing your quiz questions." }
            div {
                class: if dragging() { "upload-drop-target dragging" } else { "upload-drop-target" },
                ondragenter: move |event: DragEvent| {
                    if is_file_drag(&event) {
                        event.prevent_default();
                        drag_depth += 1;
                        dragging.set(true);
                    }
                },
                ondragover: move |event: DragEvent| {
                    if is_file_drag(&event) {
                        event.prevent_default();
                        event.data_transfer().set_drop_effect("copy");
                    }
                },
                ondragleave: move |_| {
                    let next = drag_depth.peek().saturating_sub(1);
                    drag_depth.set(next);
                    if next == 0 {
                        dragging.set(false);
                    }
                },
                ondrop: move |event: DragEvent| {
                    dragging.set(false);
                    drag_depth.set(0);
                    if is_file_drag(&event) {
                        event.prevent_default();
                        let files = event.files();
                        if files.is_empty() {
                            cancel_pending(pending_task);
                            upload_generation += 1;
                            pending.set(None);
                            loading.set(false);
                            load_error
                                .set(
                                    "Could not read the dropped file. Choose a JSON file instead."
                                        .into(),
                                );
                        } else {
                            upload(files);
                        }
                    }
                },
                label { r#for: "quiz-file", "Drop a JSON or ZIP quiz here, or choose a file" }
                input {
                    id: "quiz-file",
                    r#type: "file",
                    accept: ".json,.zip,application/json,application/zip",
                    onchange: move |event: FormEvent| {
                        upload(event.files());
                        // Selecting the same file again must retry after a failed import.
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(input) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("quiz-file")).and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok()) {
                                input.set_value("");
                            }
                        }
                    },
                }
            }
            if loading() { p { role: "status", "Preparing quiz and images…" } }
            if let Some((name, source, generation)) = pending.read().clone() {
                section { aria_label: "Choose a quiz",
                    h3 { "Choose a quiz" }
                    if let ImportSource::Archive(archive) = &source {
                        for candidate in archive.candidates.clone() {
                            { let name = name.clone(); let source = source.clone();
                              rsx! { button {
                                r#type: "button", disabled: loading(),
                                onclick: move |_| {
                                    if generation == *upload_generation.peek() && !*loading.peek() {
                                        finish(name.clone(), source.clone(), Some(candidate.clone()), generation);
                                    }
                                },
                                "{candidate}"
                              } }
                            }
                        }
                    }
                    button { r#type: "button", onclick: move |_| {
                        cancel_pending(pending_task);
                        upload_generation += 1;
                        pending.set(None);
                        loading.set(false);
                        load_error.set(String::new());
                    }, "Cancel selection" }
                }
            }
            if !load_error.read().is_empty() {
                div { class: "error-banner", {load_error.read().clone()} }
            }
            if results.read().is_some() {
                button {
                    class: "btn btn-secondary",
                    r#type: "button",
                    onclick: move |_| {
                        cancel_pending(pending_task);
                        upload_generation += 1;
                        pending.set(None);
                        loading.set(false);
                        load_error.set(String::new());
                        phase.set(AppPhase::Submitted);
                    },
                    "← Back to Results"
                }
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
                        p { "Use type: group for a shared prompt with a nonempty subquestions array. Each part is short_answer or multiple_choice with its own globally unique ID. Groups cannot nest or have parent marks, hints or answer options. Selecting one group includes all parts. Images work on each part and on the shared prompt; all images are centered. Use HTTPS URLs in JSON, or upload a ZIP with quiz JSON files at its root or inside one enclosing folder. Only JSON files directly in that location are quiz candidates: one loads automatically; with several, choose one from the sorted list. Root JSON files take priority; invalid root quizzes do not fall back to nested ones. Image paths are relative to that same location: photo.png or images/a/b/c/d/image.png. Image folders may be nested deeper. No images folder is required." }
                        p { "Limits: JSON 2 MiB; ZIP 50 MiB and 1,000 entries; total uncompressed files 100 MiB; each bundled image 10 MiB and 20 megapixels. PNG, JPEG, GIF and WebP are supported. Hosted images require network access; their hosts receive requests without a referrer." }
                        p { "Optional image width and height set maximum display dimensions in CSS pixels (positive integers). Set either or both; images keep their proportions, fit narrow screens and are not upscaled. Click an image to enlarge it beyond these limits. Results JSON preserves image references and dimensions; keep the original ZIP to view bundled images later." }
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
                            { field_ref_section("config", &
                                    config_fields) }
                            {field_ref_section("questions[ ]", &question_fields)}
                            {field_ref_section("images[ ]", &image_fields)}
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

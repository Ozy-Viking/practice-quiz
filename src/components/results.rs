use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::model::{
    AppPhase, QuestionStatus, QuizFile, QuizQuestion, ResultsData, SessionQuestion, results_json,
    shuffle_session,
};

fn metadata_value(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|v| !v.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

#[cfg(target_arch = "wasm32")]
struct DownloadLink {
    anchor: web_sys::HtmlAnchorElement,
    url: String,
}

#[cfg(target_arch = "wasm32")]
impl Drop for DownloadLink {
    fn drop(&mut self) {
        self.anchor.remove();
        // Keep the URL alive until the browser has consumed the download click.
        // This cleanup also runs when appending the anchor fails.
        let url = self.url.clone();
        let revoke = Closure::once_into_js(move || {
            let _ = web_sys::Url::revoke_object_url(&url);
        });
        let scheduled = web_sys::window().is_some_and(|window| {
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 1000)
                .is_ok()
        });
        if !scheduled {
            // Invoke it ourselves so the one-shot closure is also released.
            let _ = revoke
                .unchecked_ref::<js_sys::Function>()
                .call0(&JsValue::NULL);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn download_results(text: &str) -> Result<(), JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("Browser document unavailable"))?;
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("Browser document body unavailable"))?;
    let anchor = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/json;charset=utf-8");
    let parts = js_sys::Array::of1(&JsValue::from_str(text));
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)?;
    let download = DownloadLink {
        anchor,
        url: web_sys::Url::create_object_url_with_blob(&blob)?,
    };
    download.anchor.set_href(&download.url);
    download.anchor.set_download("quiz-results.json");
    body.append_child(&download.anchor)?;
    download.anchor.click();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn browser_export(action: &str, text: &str) -> bool {
    if action != "copy" {
        return download_results(text).is_ok();
    }
    let Some(window) = web_sys::window() else {
        return false;
    };
    let clipboard = window.navigator().clipboard();
    // The generated getter returns a Clipboard even when the browser property is absent.
    if clipboard.is_undefined() || clipboard.is_null() {
        return false;
    }
    wasm_bindgen_futures::JsFuture::from(clipboard.write_text(text))
        .await
        .is_ok()
}

#[cfg(not(target_arch = "wasm32"))]
async fn browser_export(_action: &str, _text: &str) -> bool {
    false
}

#[allow(clippy::too_many_arguments)]
fn export_results(
    action: &'static str,
    quiz: Signal<Option<QuizFile>>,
    file_name: Signal<String>,
    results: Signal<Option<ResultsData>>,
    active_seed: Signal<u32>,
    session: Signal<Vec<SessionQuestion>>,
    mut busy: Signal<bool>,
    mut feedback: Signal<String>,
    mut failed: Signal<bool>,
) {
    feedback.set(String::new());
    failed.set(false);
    let json = match (quiz.read().as_ref(), results.read().as_ref()) {
        (Some(quiz), Some(results)) => results_json(
            quiz,
            &file_name.read(),
            *active_seed.read(),
            &session.read(),
            results,
        ),
        _ => {
            failed.set(true);
            feedback.set("No completed results to export.".into());
            return;
        }
    };
    let Ok(json) = json else {
        failed.set(true);
        feedback.set("Could not prepare results JSON. Please try again.".into());
        return;
    };
    busy.set(true);
    spawn(async move {
        let success = browser_export(action, &json).await;
        busy.set(false);
        failed.set(!success);
        feedback.set(match (action, success) {
            ("copy", true) => "Results JSON copied to clipboard.",
            ("copy", false) => "Could not copy results. Check clipboard permissions or download the JSON instead.",
            (_, true) => "Download started. Check your browser's downloads for quiz-results.json.",
            (_, false) => "Could not start the download. Please try again or copy the JSON instead.",
        }.into());
    });
}

#[component]
pub fn ResultsView(
    mut phase: Signal<AppPhase>,
    mut quiz: Signal<Option<QuizFile>>,
    mut session: Signal<Vec<SessionQuestion>>,
    mut selections: Signal<HashMap<String, HashSet<String>>>,
    mut results: Signal<Option<ResultsData>>,
    mut question_count: Signal<usize>,
    mut file_name: Signal<String>,
    mut load_error: Signal<String>,
    question_pool: Signal<Vec<QuizQuestion>>,
    active_seed: Signal<u32>,
) -> Element {
    let busy = use_signal(|| false);
    let feedback = use_signal(String::new);
    let failed = use_signal(|| false);
    let mut keep_order = use_signal(|| true);
    let (res_list, total_score, total_max) =
        results
            .read()
            .as_ref()
            .cloned()
            .unwrap_or((Vec::new(), 0.0, 0.0));
    let pct = if total_max > 0.0 {
        (total_score / total_max * 100.0).round() as i32
    } else {
        0
    };
    let study_items: Vec<_> = res_list
        .iter()
        .enumerate()
        .filter(|(_, qr)| qr.score < qr.max_score || !qr.wrong_selected_answers.is_empty())
        .collect();
    let grade_class = if pct >= 80 {
        "grade-excellent"
    } else if pct >= 60 {
        "grade-good"
    } else if pct >= 40 {
        "grade-fair"
    } else {
        "grade-poor"
    };
    rsx! {
        div { class: "card results-card",
            div { class: "score-banner {grade_class}",
                h2 { "Final Score" }
                div { class: "score-number", "{total_score:.1} / {total_max:.0}" }
                div { class: "score-percent", "{pct}%" }
                super::SeedDisplay { seed: *active_seed.read() }

            }
            div { class: "results-export",
                div { class: "button-row",
                    button {
                        class: "btn btn-secondary",
                        r#type: "button",
                        disabled: busy(),
                        onclick: move |_| export_results("copy", quiz, file_name, results, active_seed, session, busy, feedback, failed),
                        "Copy Results JSON"
                    }
                    button {
                        class: "btn btn-secondary",
                        r#type: "button",
                        disabled: busy(),
                        onclick: move |_| export_results("download", quiz, file_name, results, active_seed, session, busy, feedback, failed),
                        "Download Results JSON"
                    }
                }
                if !feedback.read().is_empty() {
                    p {
                        class: if failed() { "error-banner" } else { "hint" },
                        role: "status",
                        "{feedback}"
                    }
                }
            }
            if !study_items.is_empty() {
                div { class: "study-summary",
                    div { class: "study-summary-header",
                        div {
                            h2 { "Study Review" }
                            p { "Questions to revisit from this attempt." }
                        }
                        span { class: "study-summary-count", "{study_items.len()} to review" }
                    }
                    div { class: "study-summary-list",
                        for (idx, qr) in study_items {
                            {
                                let meta = &qr.metadata;
                                rsx! {
                                    div { class: "study-summary-item",
                                        div { class: "study-summary-topline",
                                            a { class: "study-summary-question", href: "#result-q-{idx + 1}", "Q{idx + 1}: {qr.id}" }
                                            span { class: "study-summary-score", "{qr.score:.2}/{qr.max_score:.1}" }
                                        }
                                        p { class: "study-summary-text", "{qr.text}" }
                                        div { class: "study-summary-meta",
                                            if let Some(topic) = metadata_value(&meta.topic) {
                                                span { class: "study-summary-topic", "Topic: {topic}" }
                                            }
                                            if let Some(loc) = metadata_value(&meta.study_location) {
                                                span { class: "study-summary-location", "Location: {loc}" }
                                            }
                                            if let Some(timestamp) = metadata_value(&meta.timestamp) {
                                                span { "Timestamp: {timestamp}" }
                                            }
                                            if let Some(answer) = metadata_value(&meta.answer) {
                                                span { "Answer key: {answer}" }
                                            }
                                            if !meta.has_content() {
                                                span { "No study metadata provided" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for (idx, qr) in res_list.iter().enumerate() {
                {
                    let status_class = match qr.status {
                        QuestionStatus::Correct => "status-correct",
                        QuestionStatus::PartiallyCorrect => "status-partial",
                        QuestionStatus::Incorrect => "status-incorrect",
                        QuestionStatus::Unanswered => "status-unanswered",
                    };
                    let status_text = match qr.status {
                        QuestionStatus::Correct => "Correct",
                        QuestionStatus::PartiallyCorrect => "Partial",
                        QuestionStatus::Incorrect => "Incorrect",
                        QuestionStatus::Unanswered => "Unanswered",
                    };
                    rsx! {
                        div {
                            class: "result-block {status_class}",
                            key: "{qr.id}",
                            id: "result-q-{idx + 1}",
                            div { class: "result-header",
                                span { class: "result-number", "Q{idx + 1}" }
                                span { class: "result-badge {status_class}", "{status_text}" }
                                span { class: "result-score", "{qr.score:.2}/{qr.max_score:.1}" }
                            }
                            p { class: "result-question", "{qr.text}" }
                            div { class: "result-details",
                                if !qr.correct_answers.is_empty() {
                                    div { class: "detail-row correct-row",
                                        span { class: "detail-label", "Correct:" }
                                        div { class: "detail-values",
                                            for (label, text) in qr.correct_answers.iter() {
                                                div { class: "answer-entry badge-correct",
                                                    span { class: "answer-label", "{label}" }
                                                    span { class: "answer-text", "{text}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                if !qr.missed_correct_answers.is_empty() {
                                    div { class: "detail-row missed-row",
                                        span { class: "detail-label", "Missed:" }
                                        div { class: "detail-values",
                                            for (label, text) in qr.missed_correct_answers.iter() {
                                                div { class: "answer-entry badge-missed",
                                                    span { class: "answer-label", "{label}" }
                                                    span { class: "answer-text", "{text}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                if !qr.wrong_selected_answers.is_empty() {
                                    div { class: "detail-row wrong-row",
                                        span { class: "detail-label", "Wrong:" }
                                        div { class: "detail-values",
                                            for (label, text) in qr.wrong_selected_answers.iter() {
                                                div { class: "answer-entry badge-wrong",
                                                    span { class: "answer-label", "{label}" }
                                                    span { class: "answer-text", "{text}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                if !qr.selected_answers.is_empty() {
                                    div { class: "detail-row selected-row",
                                        span { class: "detail-label", "You chose:" }
                                        div { class: "detail-values",
                                            for (label, text) in qr.selected_answers.iter() {
                                                div { class: "answer-entry badge-selected",
                                                    span { class: "answer-label", "{label}" }
                                                    span { class: "answer-text", "{text}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            {
                                let meta = &qr.metadata;
                                let has_meta = meta.has_content();
                                if has_meta {
                                    rsx! {
                                        div { class: "study-info",
                                            div { class: "study-info-header", "Study Info" }
                                            if let Some(loc) = metadata_value(&meta.study_location) {
                                                p { class: "study-location", "Location: {loc}" }
                                            }
                                            if let Some(timestamp) = metadata_value(&meta.timestamp) {
                                                p { class: "study-location", "Timestamp: {timestamp}" }
                                            }
                                            if let Some(topic) = metadata_value(&meta.topic) {
                                                p { class: "study-topic", "Topic: {topic}" }
                                            }
                                            if let Some(answer) = metadata_value(&meta.answer) {
                                                p { class: "study-answer", "Original answer key: {answer}" }
                                            }
                                            if let Some(explanation) = metadata_value(&meta.explanation) {
                                                p { class: "study-notes", "Explanation: {explanation}" }
                                            } else if let Some(notes) = metadata_value(&meta.notes) {
                                                p { class: "study-notes", "Notes: {notes}" }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {
                                        div {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "rerun-actions",
                label { class: "rerun-order",
                    input {
                        r#type: "checkbox",
                        checked: *keep_order.read(),
                        onchange: move |evt| keep_order.set(evt.checked()),
                    }
                    "Keep question and answer order"
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        if session.read().is_empty() { return; }
                        if !*keep_order.read() {
                            let mut shuffled = session.read().clone();
                            shuffle_session(&mut shuffled, &mut rand::thread_rng());
                            session.set(shuffled);
                        }
                        selections.set(HashMap::new());
                        results.set(None);
                        phase.set(AppPhase::InProgress);
                    },
                    "Rerun same questions"
                }
            }
            div { class: "button-row",
                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        phase.set(AppPhase::Upload);
                        quiz.set(None);
                        session.set(Vec::new());
                        selections.set(HashMap::new());
                        results.set(None);
                        file_name.set(String::new());
                        load_error.set(String::new());
                        question_count.set(0);
                    },
                    "← Load New Quiz"
                }
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| {
                        selections.set(HashMap::new());
                        results.set(None);
                        session.set(Vec::new());
                        phase.set(AppPhase::Configuring);
                    },
                    "← Back to Settings"
                }
            }
        }
    }
}

use crate::model::{
    AppPhase, ChoiceResult, HintUsage, QuestionStatus, QuizFile, QuizQuestion, Responses,
    ResultDetails, ResultsData, SessionQuestion, results_json, shuffle_session,
};
use dioxus::prelude::*;
use std::collections::HashMap;

/// Shows every multiple-choice option with its selection and answer-key feedback.
///
/// `result` must use the same question and option ordering as `question`, because
/// selections are matched by answer label. Selection indicators retain accessible
/// names, while correct answers and mistakes receive visible status labels.
#[component]
fn AnswerReview(question: SessionQuestion, result: ChoiceResult) -> Element {
    rsx! {
        div { class: "result-details answer-review",
            h3 { "All answer options" }
            p { class: "answer-review-help", "Compare your selections with the answer key." }
            div { class: "answer-review-columns", aria_hidden: "true",
                span { "Option" }
                span { "Your answer" }
                span { "Answer key" }
            }
            ol { class: "answer-review-list", role: "list", aria_label: "All answer options",
                for (id, label, text) in &question.options {
                    {
                        let selected = result.selected_answers.iter().any(|(l, _)| l == label);
                        let correct = question.correct_ids.contains(id);
                        let state = match (selected, correct) {
                            (true, true) => "review-correct",
                            (false, true) => "review-missed",
                            (true, false) => "review-wrong",
                            (false, false) => "review-skipped",
                        };
                        rsx! {
                            li { class: "answer-review-row {state}", key: "{id}",
                                div { class: "answer-review-option",
                                    span { class: "answer-label", "{label}" }
                                    span { class: "answer-text", "{text}" }
                                }
                                div { class: "review-selection",
                                    span { class: "review-field-label", "Your answer: " }
                                    span { class: if selected { "review-choice is-selected" } else { "review-choice" },
                                        role: "img",
                                        aria_label: if selected { "Selected" } else { "Not selected" },
                                        svg {
                                            class: "review-checkbox",
                                            width: "16", height: "16", view_box: "0 0 16 16",
                                            fill: "none", stroke: "currentColor", stroke_width: "1.5",
                                            "aria-hidden": "true",
                                            rect { x: "1.5", y: "1.5", width: "13", height: "13", rx: "3" }
                                            if selected {
                                                path { d: "m4.5 8 2.25 2.25 4.75-4.75", stroke_linecap: "round", stroke_linejoin: "round" }
                                            }
                                        }
                                    }
                                }
                                div { class: "review-key",
                                    if correct || selected {
                                        span { class: "review-field-label", "Answer key: " }
                                    }
                                    if correct {
                                        span { class: "review-key-correct", "Correct answer" }
                                    }
                                    if !selected && correct {
                                        span { class: "review-outcome", "Missed" }
                                    } else if selected && !correct {
                                        span { class: "review-outcome", "Incorrect selection" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

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

/// Displays a completed attempt with scores, study guidance, and answer review.
///
/// Choice reviews use matching session questions to preserve the attempt's option
/// order; short answers remain unmarked. Exports describe the completed attempt,
/// and rerunning clears its responses, hint usage, and results before restarting.
#[component]
pub fn ResultsView(
    mut phase: Signal<AppPhase>,
    mut quiz: Signal<Option<QuizFile>>,
    mut session: Signal<Vec<SessionQuestion>>,
    mut selections: Signal<Responses>,
    mut hint_usage: Signal<HintUsage>,
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
        .filter(|(_, qr)| {
            qr.choice().is_some_and(|choice| {
                choice.score < qr.max_score || !choice.wrong_selected_answers.is_empty()
            })
        })
        .collect();
    let score_text = if total_score.fract() == 0.0 {
        format!("{total_score:.1}")
    } else {
        total_score.to_string()
    };
    let scored_count = res_list.iter().filter(|qr| qr.choice().is_some()).count();
    let total_available: f64 = res_list.iter().map(|qr| qr.max_score).sum();
    let answered_count = res_list
        .iter()
        .filter(|qr| {
            !matches!(
                qr.status,
                QuestionStatus::Unanswered | QuestionStatus::UnansweredUnmarked
            )
        })
        .count();
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
            if scored_count > 0 {
                div { class: "score-banner {grade_class}",
                    h2 {
                        if scored_count < res_list.len() {
                            "Multiple-choice Score"
                        } else {
                            "Final Score"
                        }
                    }
                    div { class: "score-number", "{score_text} / {total_max}" }
                    div { class: "score-percent", "{pct}%" }
                }
            } else {
                h2 { "Quiz Complete" }
            }
            p { class: "completion-summary", "{answered_count}/{res_list.len()} answered" }
            p { class: "total-available-marks", "Total available marks: {total_available}" }
            super::SeedDisplay { seed: *active_seed.read() }
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
                                let choice = qr.choice().unwrap();
                                rsx! {
                                    div { class: "study-summary-item",
                                        div { class: "study-summary-topline",
                                            a { class: "study-summary-question", href: "#result-q-{idx + 1}", "Q{idx + 1}: {qr.id}" }
                                            span { class: "study-summary-score", "{choice.score}/{qr.max_score}" }
                                        }
                                        p { class: "study-summary-text", "{qr.text}" }
                                        super::QuestionImages { images: qr.images.clone() }
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
                        QuestionStatus::AnsweredUnmarked | QuestionStatus::UnansweredUnmarked => {
                            "status-unmarked"
                        }
                    };
                    let status_text = match qr.status {
                        QuestionStatus::Correct => "Correct",
                        QuestionStatus::PartiallyCorrect => "Partial",
                        QuestionStatus::Incorrect => "Incorrect",
                        QuestionStatus::Unanswered => "Unanswered",
                        QuestionStatus::AnsweredUnmarked => "Answered — unmarked",
                        QuestionStatus::UnansweredUnmarked => "Unanswered — unmarked",
                    };
                    rsx! {
                        div {
                            class: "result-block {status_class}",
                            key: "{qr.id}",
                            id: "result-q-{idx + 1}",
                            div { class: "result-header",
                                span { class: "result-number", "Q{idx + 1}" }
                                span { class: "result-badge {status_class}", "{status_text}" }
                                if let Some(choice) = qr.choice() {
                                    span { class: "result-score", "{choice.score}/{qr.max_score}" }
                                }
                            }
                            p { class: "result-question", "{qr.text}" }
                            super::QuestionImages { images: qr.images.clone() }
                            p { class: "available-marks", "Available marks: {qr.max_score}" }
                            if !qr.hints.is_empty() {
                                p { class: "hint-usage",
                                    if qr.viewed_hint_indices.is_empty() {
                                        "No hints viewed"
                                    } else {
                                        "Hints viewed: {qr.viewed_hint_indices.len()} of {qr.hints.len()}"
                                    }
                                }
                            }
                            if let ResultDetails::ShortAnswer { response } = &qr.details {
                                div { class: "short-answer-response", "{response}" }
                            }
                            if let Some(choice) = qr.choice()
                                && let Some(question) = session.read().iter().find(|q| q.id == qr.id)
                            {
                                AnswerReview { question: question.clone(), result: choice.clone() }
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
                        if session.read().is_empty() {
                            return;
                        }
                        if !*keep_order.read() {
                            let mut shuffled = session.read().clone();
                            shuffle_session(&mut shuffled, &mut rand::thread_rng());
                            session.set(shuffled);
                        }
                        selections.set(HashMap::new());
                        hint_usage.set(HashMap::new());
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
                        load_error.set(String::new());
                    },
                    "← Load New Quiz"
                }
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| {
                        selections.set(HashMap::new());
                        hint_usage.set(HashMap::new());
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

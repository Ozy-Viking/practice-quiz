use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::model::{
    AppPhase, QuestionStatus, QuizFile, QuizQuestion, ResultsData, SessionQuestion,
};

fn metadata_value(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|v| !v.trim().is_empty())
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
) -> Element {
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
                                            a {
                                                class: "study-summary-question",
                                                href: "#result-q-{idx + 1}",
                                                "Q{idx + 1}: {qr.id}"
                                            }
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
                        div { class: "result-block {status_class}",
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
                                    rsx! { div {} }
                                }
                            }
                        }
                    }
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

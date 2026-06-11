use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::model::{
    build_session, AppPhase, QuizFile, QuizQuestion, ResultsData, QuestionStatus, SessionQuestion,
};

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
                                            if let Some(loc) = &meta.study_location {
                                                p { class: "study-location", "Location: {loc}" }
                                            }
                                            if let Some(timestamp) = &meta.timestamp {
                                                p { class: "study-location", "Timestamp: {timestamp}" }
                                            }
                                            if let Some(topic) = &meta.topic {
                                                p { class: "study-topic", "Topic: {topic}" }
                                            }
                                            if let Some(answer) = &meta.answer {
                                                p { class: "study-answer", "Original answer key: {answer}" }
                                            }
                                            if let Some(explanation) = &meta.explanation {
                                                p { class: "study-notes", "Explanation: {explanation}" }
                                            } else if let Some(notes) = &meta.notes {
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
                        let pool = question_pool.read().clone();
                        if pool.is_empty() {
                            return;
                        }
                        let count = *question_count.read();
                        let mut rng = rand::thread_rng();
                        let sess = build_session(&pool, count, &mut rng);
                        session.set(sess);
                        selections.set(HashMap::new());
                        results.set(None);
                        phase.set(AppPhase::InProgress);
                    },
                    "🔄 Retry Same Quiz"
                }
            }
        }
    }
}

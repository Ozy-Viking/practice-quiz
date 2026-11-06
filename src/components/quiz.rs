use crate::model::{
    AppPhase, HintUsage, QuestionType, QuizFile, Response, Responses, ResultsData, SessionQuestion,
    build_results, record_hint_view,
};
use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

#[component]
pub fn QuizView(
    mut phase: Signal<AppPhase>,
    quiz: Signal<Option<QuizFile>>,
    session: Signal<Vec<SessionQuestion>>,
    mut selections: Signal<Responses>,
    mut hint_usage: Signal<HintUsage>,
    mut results: Signal<Option<ResultsData>>,
    active_seed: Signal<u32>,
) -> Element {
    let mut expanded: Signal<HintUsage> = use_signal(HashMap::new);
    let sess = session.read();
    let config = quiz
        .read()
        .as_ref()
        .map(|q| q.effective_config())
        .unwrap_or_default();
    let scoring_config = config.clone();
    let current_selections = selections.read();
    let answered_count = current_selections
        .iter()
        .filter(|(_, v)| v.is_answered())
        .count();
    let quiz_title = quiz
        .read()
        .as_ref()
        .map(|q| q.display_title())
        .unwrap_or_default();
    rsx! {
        div { class: "card quiz-card",
            div { class: "quiz-header",
                h2 {
                    id: "quiz-heading",
                    tabindex: "-1",
                    onmounted: move |evt| async move {
                        let _ = evt.set_focus(true).await;
                        let _ = evt.scroll_to(ScrollBehavior::Instant).await;
                    },
                    "{quiz_title}"
                }
                span { class: "question-counter", "{answered_count}/{sess.len()} answered" }
            }
            super::SeedDisplay { seed: *active_seed.read() }
            for (idx, sq) in sess.iter().enumerate() {
                {
                    let qid = sq.id.clone();
                    let response = current_selections.get(&qid);
                    let selected = match response {
                        Some(Response::Choices(ids)) => ids.clone(),
                        _ => HashSet::new(),
                    };
                    let text = match response {
                        Some(Response::Text(text)) => text.clone(),
                        _ => String::new(),
                    };
                    let is_answered = response.is_some_and(Response::is_answered);
                    let textarea_qid = qid.clone();
                    rsx! {
                        div {
                            class: if is_answered { "question-block answered" } else { "question-block" },
                            key: "{qid}",
                            div { class: "question-number", "Q{idx + 1}" }
                            p { class: "question-text", id: "question-{idx}", "{sq.text}" }
                            p { class: "available-marks", "Available marks: {sq.marks}" }
                            if sq.kind == QuestionType::ShortAnswer {
                                span { class: "unmarked-label", "Unmarked" }
                                label { r#for: "response-{idx}", "Your response" }
                                textarea {
                                    id: "response-{idx}",
                                    aria_labelledby: "question-{idx}",
                                    class: "short-answer-input",
                                    rows: "5",
                                    value: "{text}",
                                    oninput: move |event| {
                                        selections.write().insert(textarea_qid.clone(), Response::Text(event.value()));
                                    },
                                }
                            } else {
                                div { class: "options-grid",
                                    for (opt_id, label, text) in sq.options.iter() {
                                        {
                                            let opt_id_clone = opt_id.clone();
                                            let qid_clone = qid.clone();
                                            let is_selected = selected.contains(opt_id);
                                            rsx! {
                                                button {
                                                    key: "{opt_id}",
                                                    class: if is_selected { "option-btn selected" } else { "option-btn" },
                                                    onclick: move |_| {
                                                        let mut sels = selections.read().clone();
                                                        let response = sels
                                                            .entry(qid_clone.clone())
                                                            .or_insert_with(|| Response::Choices(HashSet::new()));
                                                        let Response::Choices(entry) = response else {
                                                            return;
                                                        };
                                                        if entry.contains(&opt_id_clone) {
                                                            entry.remove(&opt_id_clone);
                                                        } else {
                                                            entry.insert(opt_id_clone.clone());
                                                        }
                                                        selections.set(sels);
                                                    },
                                                    span { class: "option-label", "{label}" }
                                                    span { class: "option-text", "{text}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            for (hint_index, hint) in sq.hints.iter().enumerate() {
                                {
                                    let hint_qid = qid.clone();
                                    let is_expanded = expanded
                                        .read()
                                        .get(&qid)
                                        .is_some_and(|indices| indices.contains(&hint_index));
                                    rsx! {
                                        div { class: "question-hint",
                                            button {
                                                r#type: "button",
                                                class: "btn btn-secondary hint-toggle",
                                                aria_expanded: "{is_expanded}",
                                                aria_controls: "hint-{idx}-{hint_index}",
                                                onclick: move |_| {
                                                    if is_expanded {
                                                        if let Some(indices) = expanded.write().get_mut(&hint_qid) {
                                                            indices.remove(&hint_index);
                                                        }
                                                    } else {
                                                        expanded.write().entry(hint_qid.clone()).or_default().insert(hint_index);
                                                        record_hint_view(
                                                            &session.read(),
                                                            &mut hint_usage.write(),
                                                            &hint_qid,
                                                            hint_index,
                                                        );
                                                    }
                                                },
                                                if is_expanded {
                                                    "Hide hint {hint_index + 1}"
                                                } else {
                                                    "Show hint {hint_index + 1}"
                                                }
                                            }
                                            p {
                                                id: "hint-{idx}-{hint_index}",
                                                class: "hint-text",
                                                hidden: !is_expanded,
                                                if is_expanded {
                                                    "{hint}"
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
            div { class: "button-row submit-row",
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| {
                        phase.set(AppPhase::Configuring);
                        session.set(Vec::new());
                        selections.set(HashMap::new());
                        hint_usage.set(HashMap::new());
                    },
                    "← Back to Settings"
                }
                button {
                    class: "btn btn-submit",
                    onclick: move |_| {
                        let sess = session.read().clone();
                        let sels = selections.read().clone();
                        let (res, total, max) = build_results(
                            &sess,
                            &sels,
                            &hint_usage.read(),
                            &scoring_config,
                        );
                        results.set(Some((res, total, max)));
                        phase.set(AppPhase::Submitted);
                    },
                    "Submit Answers ✓"
                }
            }
        }
    }
}

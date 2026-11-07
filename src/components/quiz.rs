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
                            super::QuestionImages { images: sq.images.clone() }
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
                            if !sq.hints.is_empty() {
                                {
                                    let hint_qid = qid.clone();
                                    let usage = hint_usage.read();
                                    let viewed = usage.get(&qid);
                                    let has_next = (0..sq.hints.len())
                                        .any(|index| !viewed.is_some_and(|indices| indices.contains(&index)));
                                    rsx! {
                                        div { class: "question-hint",
                                            div { id: "hints-{idx}", aria_live: "polite",
                                                for (hint_index, hint) in sq.hints.iter().enumerate() {
                                                    if viewed.is_some_and(|indices| indices.contains(&hint_index)) {
                                                        p { class: "hint-text", "{hint}" }
                                                    }
                                                }
                                            }
                                            if has_next {
                                                button {
                                                    r#type: "button",
                                                    class: "btn btn-secondary hint-toggle",
                                                    aria_controls: "hints-{idx}",
                                                    onclick: move |_| {
                                                        let session = session.read();
                                                        let mut usage = hint_usage.write();
                                                        if let Some(question) = session.iter().find(|q| q.id == hint_qid) {
                                                            let next = (0..question.hints.len()).find(|index| {
                                                                !usage.get(&hint_qid).is_some_and(|indices| indices.contains(index))
                                                            });
                                                            if let Some(index) = next {
                                                                record_hint_view(&session, &mut usage, &hint_qid, index);
                                                            }
                                                        }
                                                    },
                                                    "Show next hint"
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

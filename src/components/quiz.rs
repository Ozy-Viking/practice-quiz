use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::model::{AppPhase, QuizFile, ResultsData, SessionQuestion, build_results};

#[component]
pub fn QuizView(
    mut phase: Signal<AppPhase>,
    quiz: Signal<Option<QuizFile>>,
    session: Signal<Vec<SessionQuestion>>,
    mut selections: Signal<HashMap<String, HashSet<String>>>,
    mut results: Signal<Option<ResultsData>>,
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
        .filter(|(_, v)| !v.is_empty())
        .count();
    let quiz_title = quiz
        .read()
        .as_ref()
        .map(|q| q.display_title())
        .unwrap_or_default();
    rsx! {
        div { class: "card quiz-card",
            div { class: "quiz-header",
                h2 { "{quiz_title}" }
                span { class: "question-counter", "{answered_count}/{sess.len()} answered" }
            }
            for (idx, sq) in sess.iter().enumerate() {
                {
                    let qid = sq.id.clone();
                    let selected = current_selections.get(&qid).cloned().unwrap_or_default();
                    let is_answered = !selected.is_empty();
                    rsx! {
                        div {
                            class: if is_answered { "question-block answered" } else { "question-block" },
                            key: "{qid}",
                            div { class: "question-number", "Q{idx + 1}" }
                            p { class: "question-text", "{sq.text}" }
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
                                                    let entry = sels.entry(qid_clone.clone()).or_default();
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
                    },
                    "← Restart"
                }
                button {
                    class: "btn btn-submit",
                    onclick: move |_| {
                        let sess = session.read().clone();
                        let sels = selections.read().clone();
                        let (res, total, max) = build_results(&sess, &sels, &scoring_config);
                        results.set(Some((res, total, max)));
                        phase.set(AppPhase::Submitted);
                    },
                    "Submit Answers ✓"
                }
            }
        }
    }
}

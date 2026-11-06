use dioxus::prelude::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};

use crate::model::{
    AppPhase, HintUsage, QuizFile, QuizQuestion, Responses, ResultsData, SessionQuestion,
    available_locations, available_topics, build_seeded_session, parse_seed,
};

use super::FilterSection;

#[component]
pub fn ConfiguringView(
    mut phase: Signal<AppPhase>,
    mut quiz: Signal<Option<QuizFile>>,
    mut session: Signal<Vec<SessionQuestion>>,
    mut selections: Signal<Responses>,
    mut hint_usage: Signal<HintUsage>,
    mut question_count: Signal<usize>,
    file_name: Signal<String>,
    load_error: Signal<String>,
    mut question_pool: Signal<Vec<QuizQuestion>>,
    mut selected_topics: Signal<HashSet<String>>,
    selected_locations: Signal<HashSet<String>>,
    topic_regex_input: Signal<String>,
    location_regex_input: Signal<String>,
    mut seed_input: Signal<String>,
    mut active_seed: Signal<u32>,
    mut results: Signal<Option<ResultsData>>,
) -> Element {
    let q = quiz.read().as_ref().unwrap().clone();
    let config = q.effective_config();
    let allow_neg = config.allow_negative_mark;
    let marks_per_question = config.marks_per_question;

    let total_q = q.questions.len();
    let all_topics = available_topics(&q);
    let all_locations = available_locations(&q);

    // Keep the initial location selection so remounting preserves restored topic choices.
    let mut previous_locations = use_signal(|| selected_locations.peek().clone());

    // One-way sync: when location selection changes, update topic selection to only the topics
    // that exist in those locations. This effect subscribes to selected_locations only — it never
    // reads selected_topics — so writing selected_topics here cannot re-trigger this effect.
    {
        let q2 = q.clone();
        let all_locs2 = all_locations.clone();
        let all_topics2 = all_topics.clone();
        use_effect(move || {
            let sl = selected_locations.read().clone();
            if sl == *previous_locations.peek() {
                return;
            }
            previous_locations.set(sl.clone());
            let new_selection: HashSet<String> = if sl.is_empty() || sl.len() == all_locs2.len() {
                all_topics2.iter().cloned().collect()
            } else {
                q2.questions
                    .iter()
                    .filter(|qq| {
                        qq.metadata
                            .study_location
                            .as_ref()
                            .is_none_or(|l| sl.contains(l))
                    })
                    .filter_map(|qq| qq.metadata.topic.clone())
                    .collect()
            };
            selected_topics.set(new_selection);
        });
    }

    let topic_re_str = topic_regex_input.read().clone();
    let location_re_str = location_regex_input.read().clone();
    let topic_re_result: Option<Result<Regex, _>> =
        (!topic_re_str.is_empty()).then(|| Regex::new(&topic_re_str));
    let location_re_result: Option<Result<Regex, _>> =
        (!location_re_str.is_empty()).then(|| Regex::new(&location_re_str));
    let topic_re_valid = topic_re_result.as_ref().is_none_or(|r| r.is_ok());
    let location_re_valid = location_re_result.as_ref().is_none_or(|r| r.is_ok());
    let topic_re: Option<Regex> = topic_re_result.and_then(|r| r.ok());
    let location_re: Option<Regex> = location_re_result.and_then(|r| r.ok());
    let topic_regex_active = topic_re.is_some();
    let location_regex_active = location_re.is_some();

    let st = selected_topics.read().clone();
    let sl = selected_locations.read().clone();
    let all_topics_selected = st.len() == all_topics.len();
    let all_locs_selected = sl.len() == all_locations.len();

    let filtered_questions: Vec<QuizQuestion> = q
        .questions
        .iter()
        .filter(|q| {
            let topic_ok = match &topic_re {
                Some(re) => q.metadata.topic.as_ref().is_some_and(|t| re.is_match(t)),
                None => {
                    all_topics_selected || q.metadata.topic.as_ref().is_none_or(|t| st.contains(t))
                }
            };
            let loc_ok = match &location_re {
                Some(re) => q
                    .metadata
                    .study_location
                    .as_ref()
                    .is_some_and(|l| re.is_match(l)),
                None => {
                    all_locs_selected
                        || q.metadata
                            .study_location
                            .as_ref()
                            .is_none_or(|l| sl.contains(l))
                }
            };
            topic_ok && loc_ok
        })
        .cloned()
        .collect();

    let topic_matched_chips: HashSet<String> = match &topic_re {
        Some(re) => all_topics
            .iter()
            .filter(|t| re.is_match(t))
            .cloned()
            .collect(),
        None => HashSet::new(),
    };
    let location_matched_chips: HashSet<String> = match &location_re {
        Some(re) => all_locations
            .iter()
            .filter(|l| re.is_match(l))
            .cloned()
            .collect(),
        None => HashSet::new(),
    };

    let filtered_max = filtered_questions.len();
    let effective_count = if filtered_max == 0 {
        0
    } else {
        (*question_count.read()).clamp(1, filtered_max)
    };
    let seed_result = parse_seed(&seed_input.read());
    let can_start = filtered_max > 0 && seed_result.is_ok();

    rsx! {
        div { class: "card config-card",
            h2 { "{q.display_title()}" }
            p { class: "file-label", "Loaded: {file_name.read()} ({total_q} questions)" }
            if allow_neg {
                p { class: "config-flag negative-enabled", "⚠ Negative marking enabled" }
            } else {
                p { class: "config-flag negative-disabled",
                    "Negative marking disabled (scores clamped at 0)"
                }
            }
            p { class: "mark-value", "Default marks per question: {marks_per_question}. Questions can override this default." }

            if all_locations.len() >= 2 {
                FilterSection {
                    title: "Filter by Location",
                    selected: selected_locations,
                    regex_input: location_regex_input,
                    all_values: all_locations.clone(),
                    matched_chips: location_matched_chips,
                    regex_active: location_regex_active,
                    regex_valid: location_re_valid,
                    placeholder: "regex (e.g. Week [1-3])",
                }
            }

            if all_topics.len() >= 2 {
                FilterSection {
                    title: "Filter by Topic",
                    selected: selected_topics,
                    regex_input: topic_regex_input,
                    all_values: all_topics.clone(),
                    matched_chips: topic_matched_chips,
                    regex_active: topic_regex_active,
                    regex_valid: topic_re_valid,
                    placeholder: "regex (e.g. Chapter [1-3])",
                }
            }

            if filtered_max == 0 {
                div { class: "filter-empty-warning", "No questions match the current filters." }
            }

            div { class: "settings-fields",
                div { class: "config-row",
                    label { r#for: "question-count", "Questions to attempt:" }
                    input {
                        id: "question-count",
                        r#type: "number",
                        min: "1",
                        max: "{filtered_max}",
                        value: "{effective_count}",
                        disabled: filtered_max == 0,
                        onchange: {
                            let fm = filtered_max;
                            move |evt: FormEvent| {
                                let v: usize = evt.value().parse().unwrap_or(fm);
                                question_count.set(v.clamp(1, fm.max(1)));
                            }
                        },
                    }
                    span { class: "range-hint", "of {filtered_max}" }
                }
                div { class: "config-row seed-row",
                    label { r#for: "quiz-seed", "Seed:" }
                    input {
                        id: "quiz-seed",
                        r#type: "text",
                        inputmode: "numeric",
                        value: "{seed_input.read()}",
                        aria_invalid: "{seed_result.is_err()}",
                        aria_describedby: "seed-error",
                        oninput: move |evt| seed_input.set(evt.value()),
                    }
                    button {
                        class: "btn btn-secondary",
                        r#type: "button",
                        onclick: move |_| seed_input.set(rand::random::<u32>().to_string()),
                        "New seed"
                    }
                }
            }
            div { id: "seed-error", role: "alert", class: "filter-regex-error",
                if let Err(error) = seed_result { "{error}" }
            }
            div { class: "button-row",
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| {
                        phase.set(AppPhase::Upload);
                        quiz.set(None);
                        file_name.set(String::new());
                        load_error.set(String::new());
                        question_count.set(0);
                    },
                    "← Back"
                }
                button {
                    class: "btn btn-primary",
                    disabled: !can_start,
                    onclick: {
                        let fq = filtered_questions.clone();
                        move |_| {
                            let Ok(seed) = parse_seed(&seed_input.read()) else { return; };
                            if fq.is_empty() { return; }
                            let count = effective_count;
                            let sess = build_seeded_session(&fq, count, seed, config.marks_per_question);
                            seed_input.set(seed.to_string());
                            active_seed.set(seed);
                            results.set(None);
                            question_count.set(count);
                            question_pool.set(fq.clone());
                            session.set(sess);
                            selections.set(HashMap::new());
                            hint_usage.set(HashMap::new());
                            phase.set(AppPhase::InProgress);
                        }
                    },
                    "Start Quiz →"
                }
            }
        }
    }
}

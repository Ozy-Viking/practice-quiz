use dioxus::prelude::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};

use crate::model::{
    available_locations, available_topics, build_session, AppPhase, QuizFile, QuizQuestion,
    SessionQuestion,
};

use super::FilterSection;

#[component]
pub fn ConfiguringView(
    mut phase: Signal<AppPhase>,
    mut quiz: Signal<Option<QuizFile>>,
    mut session: Signal<Vec<SessionQuestion>>,
    mut selections: Signal<HashMap<String, HashSet<String>>>,
    mut question_count: Signal<usize>,
    file_name: Signal<String>,
    load_error: Signal<String>,
    mut question_pool: Signal<Vec<QuizQuestion>>,
) -> Element {
    let q = quiz.read().as_ref().unwrap().clone();
    let config = q.effective_config();
    let allow_neg = config.allow_negative_mark;
    let marks_per_question = config.marks_per_question;

    let total_q = q.questions.len();
    let all_topics = available_topics(&q);
    let all_locations = available_locations(&q);

    let init_topics: HashSet<String> = all_topics.iter().cloned().collect();
    let init_locs: HashSet<String> = all_locations.iter().cloned().collect();
    let selected_topics: Signal<HashSet<String>> = use_signal(move || init_topics);
    let selected_locations: Signal<HashSet<String>> = use_signal(move || init_locs);
    let topic_regex_input: Signal<String> = use_signal(String::new);
    let location_regex_input: Signal<String> = use_signal(String::new);

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
                    all_topics_selected
                        || q.metadata.topic.as_ref().is_none_or(|t| st.contains(t))
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
        Some(re) => all_topics.iter().filter(|t| re.is_match(t)).cloned().collect(),
        None => HashSet::new(),
    };
    let location_matched_chips: HashSet<String> = match &location_re {
        Some(re) => all_locations.iter().filter(|l| re.is_match(l)).cloned().collect(),
        None => HashSet::new(),
    };

    let filtered_max = filtered_questions.len();
    let effective_count = if filtered_max == 0 {
        0
    } else {
        (*question_count.read()).clamp(1, filtered_max)
    };
    let can_start = filtered_max > 0;

    rsx! {
        div { class: "card config-card",
            h2 { "{q.display_title()}" }
            p { class: "file-label", "Loaded: {file_name.read()} ({total_q} questions)" }
            if allow_neg {
                p { class: "config-flag negative-enabled", "⚠ Negative marking enabled" }
            } else {
                p { class: "config-flag negative-disabled", "Negative marking disabled (scores clamped at 0)" }
            }
            p { class: "mark-value", "Each question is worth {marks_per_question:.1} marks" }

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

            if !can_start {
                div { class: "filter-empty-warning", "No questions match the current filters." }
            }

            div { class: "config-row",
                label { "Questions to attempt:" }
                input {
                    r#type: "number",
                    min: "1",
                    max: "{filtered_max}",
                    value: "{effective_count}",
                    disabled: !can_start,
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
                            let count = effective_count;
                            let mut rng = rand::thread_rng();
                            let sess = build_session(&fq, count, &mut rng);
                            question_count.set(count);
                            question_pool.set(fq.clone());
                            session.set(sess);
                            selections.set(HashMap::new());
                            phase.set(AppPhase::InProgress);
                        }
                    },
                    "Start Quiz →"
                }
            }
        }
    }
}

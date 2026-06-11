mod model;

use dioxus::prelude::*;
use model::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let phase: Signal<AppPhase> = use_signal(|| AppPhase::Upload);
    let quiz: Signal<Option<QuizFile>> = use_signal(|| None);
    let session: Signal<Vec<SessionQuestion>> = use_signal(|| Vec::new());
    let selections: Signal<HashMap<String, HashSet<String>>> = use_signal(|| HashMap::new());
    let question_count: Signal<usize> = use_signal(|| 0);
    let file_name: Signal<String> = use_signal(|| String::new());
    let load_error: Signal<String> = use_signal(|| String::new());
    let results: Signal<Option<ResultsData>> = use_signal(|| None);
    let question_pool: Signal<Vec<QuizQuestion>> = use_signal(|| Vec::new());

    let current = phase.read().clone();

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        div { id: "app",
            header { class: "app-header",
                h1 { "Practice Quiz" }
                p { class: "subtitle", "Load a JSON quiz file and test your knowledge" }
            }
            if current == AppPhase::Upload {
                UploadView { phase, quiz, question_count, file_name, load_error }
            } else if current == AppPhase::Configuring {
                ConfiguringView {
                    phase,
                    quiz,
                    session,
                    selections,
                    question_count,
                    file_name,
                    load_error,
                    question_pool,
                }
            } else if current == AppPhase::InProgress {
                QuizView { phase, quiz, session, selections, results }
            } else if current == AppPhase::Submitted {
                ResultsView {
                    phase,
                    quiz,
                    session,
                    selections,
                    results,
                    question_count,
                    file_name,
                    load_error,
                    question_pool,
                }
            }
        }
    }
}

#[component]
fn UploadView(
    mut phase: Signal<AppPhase>,
    mut quiz: Signal<Option<QuizFile>>,
    mut question_count: Signal<usize>,
    mut file_name: Signal<String>,
    mut load_error: Signal<String>,
) -> Element {
    rsx! {
        div { class: "card upload-card",
            h2 { "Load Quiz File" }
            p { class: "hint",
                "Upload a JSON file with the format: "
                code { "{{ \"title\": ..., \"questions\": [...] }}" }
            }
            input {
                r#type: "file",
                accept: ".json,application/json",
                onchange: move |evt: FormEvent| {
                    let files = evt.files();
                    let file = match files.into_iter().next() {
                        Some(f) => f,
                        None => return,
                    };
                    let name = file.name();
                    spawn(async move {
                        let text = match file.read_string().await {
                            Ok(t) => t,
                            Err(e) => {
                                load_error.set(format!("Failed to read file: {e}"));
                                return;
                            }
                        };
                        let parsed: Result<QuizFile, _> = serde_json::from_str(&text);
                        match parsed {
                            Ok(q) => match validate_quiz(&q) {
                                Ok(()) => {
                                    question_count.set(initial_question_count(&q));
                                    file_name.set(name);
                                    quiz.set(Some(q));
                                    phase.set(AppPhase::Configuring);
                                    load_error.set(String::new());
                                }
                                Err(msg) => load_error.set(msg),
                            },
                            Err(e) => load_error.set(format!("Invalid JSON: {e}")),
                        }
                    });
                },
            }
            if !load_error.read().is_empty() {
                div { class: "error-banner", {load_error.read().clone()} }
            }
        }
    }
}

#[component]
fn ConfiguringView(
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
    let mut selected_topics: Signal<HashSet<String>> = use_signal(move || init_topics);
    let mut selected_locations: Signal<HashSet<String>> = use_signal(move || init_locs);
    let mut topic_regex_input: Signal<String> = use_signal(|| String::new());
    let mut location_regex_input: Signal<String> = use_signal(|| String::new());

    // Parse regex inputs — None means no regex entered, Some(Err) means invalid pattern
    let topic_re_str = topic_regex_input.read().clone();
    let location_re_str = location_regex_input.read().clone();
    let topic_re_result: Option<Result<Regex, _>> =
        (!topic_re_str.is_empty()).then(|| Regex::new(&topic_re_str));
    let location_re_result: Option<Result<Regex, _>> =
        (!location_re_str.is_empty()).then(|| Regex::new(&location_re_str));
    let topic_re_valid = topic_re_result.as_ref().map_or(true, |r| r.is_ok());
    let location_re_valid = location_re_result.as_ref().map_or(true, |r| r.is_ok());
    let topic_re: Option<Regex> = topic_re_result.and_then(|r| r.ok());
    let location_re: Option<Regex> = location_re_result.and_then(|r| r.ok());
    let topic_regex_active = topic_re.is_some();
    let location_regex_active = location_re.is_some();

    // Compute filtered question pool and effective max
    let st = selected_topics.read().clone();
    let sl = selected_locations.read().clone();
    let all_topics_selected = st.len() == all_topics.len();
    let all_locs_selected = sl.len() == all_locations.len();

    let filtered_questions: Vec<QuizQuestion> = q
        .questions
        .iter()
        .filter(|q| {
            let topic_ok = match &topic_re {
                Some(re) => q.metadata.topic.as_ref().map_or(false, |t| re.is_match(t)),
                None => {
                    all_topics_selected
                        || q.metadata.topic.as_ref().map_or(true, |t| st.contains(t))
                }
            };
            let loc_ok = match &location_re {
                Some(re) => q
                    .metadata
                    .study_location
                    .as_ref()
                    .map_or(false, |l| re.is_match(l)),
                None => {
                    all_locs_selected
                        || q.metadata
                            .study_location
                            .as_ref()
                            .map_or(true, |l| sl.contains(l))
                }
            };
            topic_ok && loc_ok
        })
        .cloned()
        .collect();

    // Which chip values match the current regex (for highlight rendering)
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

            // Topic filter
            if all_topics.len() >= 2 {
                {
                    let at_all = all_topics.clone();
                    let at_chips = all_topics.clone();
                    let at_matched = topic_matched_chips.clone();
                    rsx! {
                        div { class: "filter-section",
                            div { class: "filter-section-header",
                                span { class: "filter-section-title", "Filter by Topic" }
                                button {
                                    class: "filter-action",
                                    r#type: "button",
                                    onclick: move |_| {
                                        selected_topics.set(at_all.iter().cloned().collect());
                                    },
                                    "All"
                                }
                                button {
                                    class: "filter-action",
                                    r#type: "button",
                                    onclick: move |_| {
                                        selected_topics.set(HashSet::new());
                                    },
                                    "None"
                                }
                            }
                            div { class: "filter-regex-wrapper",
                                input {
                                    class: if topic_re_valid { "filter-regex-input" } else { "filter-regex-input invalid" },
                                    r#type: "text",
                                    placeholder: "regex (e.g. Chapter [1-3])",
                                    value: "{topic_regex_input.read()}",
                                    oninput: move |evt| topic_regex_input.set(evt.value()),
                                }
                                if !topic_re_valid {
                                    span { class: "filter-regex-error", "Invalid regex" }
                                }
                            }
                            div {
                                class: if topic_regex_active { "filter-chips regex-active" } else { "filter-chips" },
                                for topic in at_chips.iter() {
                                    {
                                        let tv = topic.clone();
                                        let is_checked = selected_topics.read().contains(&tv);
                                        let chip_class = if topic_regex_active {
                                            if at_matched.contains(&tv) { "filter-chip regex-match" } else { "filter-chip regex-no-match" }
                                        } else {
                                            "filter-chip"
                                        };
                                        rsx! {
                                            label { class: "{chip_class}", key: "{tv}",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: is_checked,
                                                    onchange: move |_| {
                                                        let mut set = selected_topics.read().clone();
                                                        if set.contains(&tv) {
                                                            set.remove(&tv);
                                                        } else {
                                                            set.insert(tv.clone());
                                                        }
                                                        selected_topics.set(set);
                                                    },
                                                }
                                                "{tv}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Location filter
            if all_locations.len() >= 2 {
                {
                    let al_all = all_locations.clone();
                    let al_chips = all_locations.clone();
                    let al_matched = location_matched_chips.clone();
                    rsx! {
                        div { class: "filter-section",
                            div { class: "filter-section-header",
                                span { class: "filter-section-title", "Filter by Location" }
                                button {
                                    class: "filter-action",
                                    r#type: "button",
                                    onclick: move |_| {
                                        selected_locations.set(al_all.iter().cloned().collect());
                                    },
                                    "All"
                                }
                                button {
                                    class: "filter-action",
                                    r#type: "button",
                                    onclick: move |_| {
                                        selected_locations.set(HashSet::new());
                                    },
                                    "None"
                                }
                            }
                            div { class: "filter-regex-wrapper",
                                input {
                                    class: if location_re_valid { "filter-regex-input" } else { "filter-regex-input invalid" },
                                    r#type: "text",
                                    placeholder: "regex (e.g. Week [1-3])",
                                    value: "{location_regex_input.read()}",
                                    oninput: move |evt| location_regex_input.set(evt.value()),
                                }
                                if !location_re_valid {
                                    span { class: "filter-regex-error", "Invalid regex" }
                                }
                            }
                            div {
                                class: if location_regex_active { "filter-chips regex-active" } else { "filter-chips" },
                                for loc in al_chips.iter() {
                                    {
                                        let lv = loc.clone();
                                        let is_checked = selected_locations.read().contains(&lv);
                                        let chip_class = if location_regex_active {
                                            if al_matched.contains(&lv) { "filter-chip regex-match" } else { "filter-chip regex-no-match" }
                                        } else {
                                            "filter-chip"
                                        };
                                        rsx! {
                                            label { class: "{chip_class}", key: "{lv}",
                                                input {
                                                    r#type: "checkbox",
                                                    checked: is_checked,
                                                    onchange: move |_| {
                                                        let mut set = selected_locations.read().clone();
                                                        if set.contains(&lv) {
                                                            set.remove(&lv);
                                                        } else {
                                                            set.insert(lv.clone());
                                                        }
                                                        selected_locations.set(set);
                                                    },
                                                }
                                                "{lv}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
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

#[component]
fn QuizView(
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

#[component]
fn ResultsView(
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
                                if !qr.correct_labels.is_empty() {
                                    div { class: "detail-row correct-row",
                                        span { class: "detail-label", "Correct:" }
                                        span { class: "detail-values",
                                            for label in qr.correct_labels.iter() {
                                                span { class: "badge badge-correct", "{label}" }
                                            }
                                        }
                                    }
                                }
                                if !qr.missed_correct_labels.is_empty() {
                                    div { class: "detail-row missed-row",
                                        span { class: "detail-label", "Missed:" }
                                        span { class: "detail-values",
                                            for label in qr.missed_correct_labels.iter() {
                                                span { class: "badge badge-missed", "{label}" }
                                            }
                                        }
                                    }
                                }
                                if !qr.wrong_selected_labels.is_empty() {
                                    div { class: "detail-row wrong-row",
                                        span { class: "detail-label", "Wrong:" }
                                        span { class: "detail-values",
                                            for label in qr.wrong_selected_labels.iter() {
                                                span { class: "badge badge-wrong", "{label}" }
                                            }
                                        }
                                    }
                                }
                                if !qr.selected_labels.is_empty() {
                                    div { class: "detail-row selected-row",
                                        span { class: "detail-label", "You chose:" }
                                        span { class: "detail-values",
                                            for label in qr.selected_labels.iter() {
                                                span { class: "badge badge-selected", "{label}" }
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

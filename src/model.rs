use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct QuizFile {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub config: QuizConfig,
    #[serde(default)]
    pub marks_per_question: Option<f64>,
    #[serde(default, alias = "negative_marks")]
    pub negative_marks: Option<bool>,
    pub questions: Vec<QuizQuestion>,
}

impl QuizFile {
    pub fn effective_config(&self) -> QuizConfig {
        let mut config = self.config.clone();
        if let Some(marks) = self.marks_per_question {
            config.marks_per_question = marks;
        }
        if let Some(negative_marks) = self.negative_marks {
            config.allow_negative_mark = negative_marks;
        }
        config
    }

    pub fn display_title(&self) -> String {
        if !self.title.trim().is_empty() {
            self.title.clone()
        } else if let Some(title) = &self.config.title {
            title.clone()
        } else {
            "Practice Quiz".to_string()
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct QuizConfig {
    #[serde(default = "default_marks_per_question")]
    pub marks_per_question: f64,
    #[serde(default, alias = "negative_marks")]
    pub allow_negative_mark: bool,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_question_count: Option<usize>,
}

impl Default for QuizConfig {
    fn default() -> Self {
        Self {
            marks_per_question: default_marks_per_question(),
            allow_negative_mark: false,
            title: None,
            description: None,
            default_question_count: None,
        }
    }
}

fn default_marks_per_question() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    #[default]
    MultipleChoice,
    ShortAnswer,
}

// Only omission selects a default; an explicit null is invalid.
fn deserialize_marks<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    f64::deserialize(d).map(Some)
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(try_from = "QuestionInput")]
pub struct QuizQuestion {
    pub id: String,
    pub question: String,
    #[serde(default, rename = "type")]
    pub kind: QuestionType,
    #[serde(
        default,
        deserialize_with = "deserialize_marks",
        skip_serializing_if = "Option::is_none"
    )]
    pub marks: Option<f64>,
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default, alias = "correctAnswers")]
    pub correct_answers: Vec<String>,
    #[serde(default, alias = "incorrectAnswers")]
    pub incorrect_answers: Vec<String>,
    #[serde(default)]
    pub explanation: Option<String>,
    #[serde(default, alias = "hiddenInfo")]
    pub metadata: QuestionMetadata,
}
// MCQs retain their required option arrays; short answers may omit them.
#[derive(Deserialize)]
struct QuestionInput {
    id: String,
    question: String,
    #[serde(default, rename = "type")]
    kind: QuestionType,
    #[serde(default, deserialize_with = "deserialize_marks")]
    marks: Option<f64>,
    #[serde(default)]
    hints: Vec<String>,
    #[serde(
        default,
        alias = "correctAnswers",
        deserialize_with = "deserialize_options"
    )]
    correct_answers: Option<Vec<String>>,
    #[serde(
        default,
        alias = "incorrectAnswers",
        deserialize_with = "deserialize_options"
    )]
    incorrect_answers: Option<Vec<String>>,
    #[serde(default)]
    explanation: Option<String>,
    #[serde(default, alias = "hiddenInfo")]
    metadata: QuestionMetadata,
}

fn deserialize_options<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Vec<String>>, D::Error> {
    Vec::<String>::deserialize(d).map(Some)
}

impl TryFrom<QuestionInput> for QuizQuestion {
    type Error = String;
    fn try_from(input: QuestionInput) -> Result<Self, Self::Error> {
        if input.kind == QuestionType::MultipleChoice
            && (input.correct_answers.is_none() || input.incorrect_answers.is_none())
        {
            return Err(format!(
                "Multiple-choice question {} requires correct_answers and incorrect_answers",
                input.id,
            ));
        }
        Ok(Self {
            id: input.id,
            question: input.question,
            kind: input.kind,
            marks: input.marks,
            hints: input.hints,
            correct_answers: input.correct_answers.unwrap_or_default(),
            incorrect_answers: input.incorrect_answers.unwrap_or_default(),
            explanation: input.explanation,
            metadata: input.metadata,
        })
    }
}

impl QuizQuestion {
    fn merged_metadata(&self) -> QuestionMetadata {
        let mut metadata = self.metadata.clone();
        if metadata.explanation.is_none() {
            metadata.explanation = self.explanation.clone();
        }
        metadata
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct QuestionMetadata {
    #[serde(default)]
    pub study_location: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub answer: Option<String>,
    #[serde(default)]
    pub explanation: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl QuestionMetadata {
    pub fn has_content(&self) -> bool {
        has_metadata_value(&self.study_location)
            || has_metadata_value(&self.timestamp)
            || has_metadata_value(&self.topic)
            || has_metadata_value(&self.answer)
            || has_metadata_value(&self.explanation)
            || has_metadata_value(&self.notes)
    }
}

fn has_metadata_value(value: &Option<String>) -> bool {
    value.as_ref().is_some_and(|v| !v.trim().is_empty())
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionQuestion {
    pub id: String,
    pub text: String,
    pub kind: QuestionType,
    pub marks: f64,
    pub hints: Vec<String>,
    pub options: Vec<(String, String, String)>,
    pub correct_ids: HashSet<String>,
    pub metadata: QuestionMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStatus {
    Correct,
    PartiallyCorrect,
    Incorrect,
    Unanswered,
    AnsweredUnmarked,
    UnansweredUnmarked,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Choices(HashSet<String>),
    Text(String),
}

impl Response {
    pub fn is_answered(&self) -> bool {
        match self {
            Self::Choices(ids) => !ids.is_empty(),
            Self::Text(text) => !text.trim().is_empty(),
        }
    }
}

pub type Responses = HashMap<String, Response>;
pub type HintUsage = HashMap<String, BTreeSet<usize>>;
pub fn record_hint_view(
    session: &[SessionQuestion],
    usage: &mut HintUsage,
    id: &str,
    index: usize,
) {
    if session.iter().any(|q| q.id == id && index < q.hints.len()) {
        usage.entry(id.to_string()).or_default().insert(index);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceResult {
    pub score: f64,
    pub selected_answers: Vec<(String, String)>,
    pub correct_answers: Vec<(String, String)>,
    pub missed_correct_answers: Vec<(String, String)>,
    pub wrong_selected_answers: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResultDetails {
    MultipleChoice(ChoiceResult),
    ShortAnswer { response: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuestionResult {
    pub id: String,
    pub text: String,
    pub status: QuestionStatus,
    pub max_score: f64,
    pub details: ResultDetails,
    pub hints: Vec<String>,
    pub viewed_hint_indices: Vec<usize>,
    pub metadata: QuestionMetadata,
}

impl QuestionResult {
    pub fn choice(&self) -> Option<&ChoiceResult> {
        match &self.details {
            ResultDetails::MultipleChoice(choice) => Some(choice),
            _ => None,
        }
    }
}

pub type ResultsData = (Vec<QuestionResult>, f64, f64);

#[derive(Serialize)]
struct QuestionExport<'a> {
    id: &'a str,
    text: &'a str,
    status: QuestionStatus,
    max_score: f64,
    metadata: &'a QuestionMetadata,
    hints: &'a [String],
    hints_viewed: bool,
    viewed_hint_indices: &'a [usize],
    #[serde(flatten)]
    details: ExportDetails<'a>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ExportDetails<'a> {
    MultipleChoice {
        score: f64,
        answers: BTreeMap<String, &'a str>,
        correct: Vec<String>,
        selected: Vec<String>,
        missed: Vec<String>,
        incorrect: Vec<String>,
    },
    ShortAnswer {
        response: &'a str,
    },
}

impl<'a> QuestionExport<'a> {
    fn new(result: &'a QuestionResult, session: &'a SessionQuestion) -> Self {
        fn labels(answers: &[(String, String)]) -> Vec<String> {
            answers
                .iter()
                .map(|(label, _)| label.to_ascii_lowercase())
                .collect()
        }
        let details = match &result.details {
            ResultDetails::ShortAnswer { response } => ExportDetails::ShortAnswer { response },
            ResultDetails::MultipleChoice(choice) => ExportDetails::MultipleChoice {
                score: choice.score,
                answers: session
                    .options
                    .iter()
                    .map(|(_, label, text)| (label.to_ascii_lowercase(), text.as_str()))
                    .collect(),
                correct: labels(&choice.correct_answers),
                selected: labels(&choice.selected_answers),
                missed: labels(&choice.missed_correct_answers),
                incorrect: labels(&choice.wrong_selected_answers),
            },
        };
        Self {
            id: &result.id,
            text: &result.text,
            status: result.status,
            max_score: result.max_score,
            metadata: &result.metadata,
            hints: &result.hints,
            hints_viewed: !result.viewed_hint_indices.is_empty(),
            viewed_hint_indices: &result.viewed_hint_indices,
            details,
        }
    }
}
/// A completed attempt, independent of the quiz input file format.
#[derive(Serialize)]
struct ResultsExport<'a> {
    format_version: u32,
    quiz_title: String,
    source_file: &'a str,
    seed: u32,
    config: QuizConfig,
    question_count: usize,
    total_score: f64,
    total_max: f64,
    percentage: Option<f64>,
    scored_question_count: usize,
    total_available_marks: f64,
    questions: Vec<QuestionExport<'a>>,
}

pub fn results_json(
    quiz: &QuizFile,
    source_file: &str,
    seed: u32,
    session: &[SessionQuestion],
    results: &ResultsData,
) -> Result<String, serde_json::Error> {
    let (questions, total_score, total_max) = results;
    if questions.len() != session.len()
        || questions
            .iter()
            .zip(session)
            .any(|(result, question)| result.id != question.id)
    {
        return Err(<serde_json::Error as serde::ser::Error>::custom(
            "Completed results do not match the session",
        ));
    }
    serde_json::to_string_pretty(&ResultsExport {
        format_version: 3,
        quiz_title: quiz.display_title(),
        source_file,
        seed,
        config: quiz.effective_config(),
        question_count: questions.len(),
        total_score: *total_score,
        total_max: *total_max,
        percentage: (*total_max > 0.0).then(|| total_score / total_max * 100.0),
        scored_question_count: questions.iter().filter(|q| q.choice().is_some()).count(),
        total_available_marks: questions.iter().map(|q| q.max_score).sum(),
        questions: questions
            .iter()
            .zip(session)
            .map(|(result, question)| QuestionExport::new(result, question))
            .collect(),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppPhase {
    Upload,
    Configuring,
    InProgress,
    Submitted,
}
/// Canonical example used in the upload screen and validated by tests.
/// The component replaces `SCHEMA_URL` with the live origin URL at render time.
pub const EXAMPLE_QUIZ_JSON: &str = r#"{
  "$schema": "SCHEMA_URL",
  "title": "My Quiz",
  "config": {
    "marks_per_question": 1.0,
    "allow_negative_mark": false,
    "default_question_count": 20
  },
  "questions": [
    {
      "id": "q1",
      "question": "Question text?",
      "correct_answers": ["Right answer"],
      "incorrect_answers": ["Wrong A", "Wrong B", "Wrong C"],
      "hints": ["Think about the definition."],
      "metadata": { "topic": "Chapter 1", "explanation": "Because..." }
    },
    {
      "id": "sa-1",
      "type": "short_answer",
      "marks": 2.5,
      "question": "Explain your reasoning.",
      "hints": ["What stays unchanged?", "Consider the starting conditions."],
      "metadata": { "topic": "Chapter 1" }
    }
  ]
}"#;

pub fn validate_quiz(quiz: &QuizFile) -> Result<(), String> {
    let config = quiz.effective_config();
    if !config.marks_per_question.is_finite() || config.marks_per_question <= 0.0 {
        return Err("marks_per_question must be greater than 0".to_string());
    }
    if quiz.questions.is_empty() {
        return Err("Quiz file contains no questions".to_string());
    }
    let mut ids = HashSet::new();
    for q in &quiz.questions {
        if !ids.insert(&q.id) {
            return Err(format!("Duplicate question id: {}", q.id));
        }
        if q.marks
            .is_some_and(|marks| !marks.is_finite() || marks <= 0.0)
        {
            return Err(format!(
                "Question {} marks must be a finite number greater than 0",
                q.id
            ));
        }
        if q.hints.iter().any(|hint| hint.trim().is_empty()) {
            return Err(format!(
                "Question {} hints must contain nonempty text",
                q.id
            ));
        }
        if q.kind == QuestionType::ShortAnswer {
            if !q.correct_answers.is_empty() || !q.incorrect_answers.is_empty() {
                return Err(format!(
                    "Short-answer question {} must not have answer options",
                    q.id,
                ));
            }
            continue;
        }
        let total = q.correct_answers.len() + q.incorrect_answers.len();
        if total == 0 {
            return Err(format!(
                "Question \"{}\" (id: {}) has no answers",
                q.question, q.id
            ));
        }
        if q.correct_answers.is_empty() {
            return Err(format!(
                "Question \"{}\" (id: {}) has no correct answers",
                q.question, q.id,
            ));
        }
    }
    Ok(())
}

pub fn initial_question_count(quiz: &QuizFile) -> usize {
    let max = quiz.questions.len();
    quiz.effective_config()
        .default_question_count
        .unwrap_or(max)
        .clamp(1, max)
}

pub fn available_topics(quiz: &QuizFile) -> Vec<String> {
    let topics: std::collections::BTreeSet<String> = quiz
        .questions
        .iter()
        .filter_map(|q| q.metadata.topic.clone())
        .collect();
    topics.into_iter().collect()
}

pub fn available_locations(quiz: &QuizFile) -> Vec<String> {
    let locs: std::collections::BTreeSet<String> = quiz
        .questions
        .iter()
        .filter_map(|q| q.metadata.study_location.clone())
        .collect();
    locs.into_iter().collect()
}
/// Generator v1: ChaCha8, rand 0.8 shuffle, source array order preserved before shuffling.
/// A fresh RNG controls both question selection and answer ordering on every call.
pub fn build_seeded_session(
    questions: &[QuizQuestion],
    count: usize,
    seed: u32,
    default_marks: f64,
) -> Vec<SessionQuestion> {
    use rand::SeedableRng;
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(u64::from(seed));
    build_session(questions, count, default_marks, &mut rng)
}

pub fn parse_seed(input: &str) -> Result<u32, &'static str> {
    let input = input.trim();
    if input.is_empty() || !input.bytes().all(|b| b.is_ascii_digit()) {
        return Err("Enter a whole number from 0 to 4294967295 using decimal digits.");
    }
    input
        .parse()
        .map_err(|_| "Enter a whole number from 0 to 4294967295 using decimal digits.")
}

pub fn build_session(
    questions: &[QuizQuestion],
    count: usize,
    default_marks: f64,
    rng: &mut impl rand::Rng,
) -> Vec<SessionQuestion> {
    use rand::seq::SliceRandom;
    let mut questions = questions.to_vec();
    questions.shuffle(rng);
    let selected: Vec<_> = questions.into_iter().take(count).collect();
    selected
        .into_iter()
        .map(|q| {
            let metadata = q.merged_metadata();
            let mut all_answers: Vec<(bool, String)> = q
                .correct_answers
                .iter()
                .map(|a| (true, a.clone()))
                .chain(q.incorrect_answers.iter().map(|a| (false, a.clone())))
                .collect();
            let is_true_false = all_answers.len() == 2 && {
                let texts: Vec<&str> = all_answers.iter().map(|(_, t)| t.as_str()).collect();
                texts.iter().any(|t| t.eq_ignore_ascii_case("true"))
                    && texts.iter().any(|t| t.eq_ignore_ascii_case("false"))
            };
            if is_true_false {
                all_answers.sort_by_key(|(_, t)| !t.eq_ignore_ascii_case("true"));
            } else {
                all_answers.shuffle(rng);
            }
            let correct_ids: HashSet<String> = all_answers
                .iter()
                .enumerate()
                .filter(|(_, (is_correct, _))| *is_correct)
                .map(|(i, _)| format!("opt_{i}"))
                .collect();
            let options: Vec<(String, String, String)> = all_answers
                .iter()
                .enumerate()
                .map(|(i, (_, text))| (format!("opt_{i}"), option_label(i), text.clone()))
                .collect();
            SessionQuestion {
                id: q.id,
                text: q.question,
                kind: q.kind,
                marks: q.marks.unwrap_or(default_marks),
                hints: q.hints,
                options,
                correct_ids,
                metadata,
            }
        })
        .collect()
}

fn option_label(mut index: usize) -> String {
    let mut label = String::new();
    loop {
        label.insert(0, (b'A' + (index % 26) as u8) as char);
        if index < 26 {
            return label;
        }
        index = index / 26 - 1;
    }
}

/// Shuffle an existing attempt without changing its questions or correct-answer identities.
pub fn shuffle_session(session: &mut [SessionQuestion], rng: &mut impl rand::Rng) {
    use rand::seq::SliceRandom;

    fn shuffle_changed<T: Clone + PartialEq>(items: &mut [T], rng: &mut impl rand::Rng) {
        let original = items.to_vec();
        items.shuffle(rng);
        if items.len() > 1 && items == original {
            items.rotate_left(1);
        }
    }

    shuffle_changed(session, rng);
    for question in session {
        let is_true_false = question.options.len() == 2
            && question
                .options
                .iter()
                .any(|(_, _, text)| text.eq_ignore_ascii_case("true"))
            && question
                .options
                .iter()
                .any(|(_, _, text)| text.eq_ignore_ascii_case("false"));
        if !is_true_false {
            shuffle_changed(&mut question.options, rng);
        }
        // IDs travel with their answers so scoring stays correct; labels follow display order.
        for (index, (_, label, _)) in question.options.iter_mut().enumerate() {
            *label = option_label(index);
        }
    }
}

pub fn score_question(
    correct_ids: &HashSet<String>,
    selected_ids: &HashSet<String>,
    total_options: usize,
    marks_per_question: f64,
    allow_negative_mark: bool,
) -> (f64, f64, QuestionStatus) {
    let n_correct = correct_ids.len();
    if selected_ids.is_empty() {
        return (0.0, marks_per_question, QuestionStatus::Unanswered);
    }
    if n_correct == 0 {
        return (0.0, marks_per_question, QuestionStatus::Incorrect);
    }

    let correct_selected = selected_ids.intersection(correct_ids).count();
    let wrong_selected = selected_ids.difference(correct_ids).count();
    let incorrect_available = total_options.saturating_sub(n_correct).max(1);
    let positive = correct_selected as f64 / n_correct as f64;
    let penalty = wrong_selected as f64 / incorrect_available as f64;
    let raw_fraction = positive - penalty;
    let fraction = if allow_negative_mark {
        raw_fraction
    } else {
        raw_fraction.max(0.0)
    };
    let score = fraction * marks_per_question;
    let status = if correct_selected == n_correct && wrong_selected == 0 {
        QuestionStatus::Correct
    } else if correct_selected > 0 {
        QuestionStatus::PartiallyCorrect
    } else {
        QuestionStatus::Incorrect
    };
    (score, marks_per_question, status)
}

pub fn build_results(
    session: &[SessionQuestion],
    selections: &Responses,
    hint_usage: &HintUsage,
    config: &QuizConfig,
) -> ResultsData {
    let mut results = Vec::new();
    let mut total_score = 0.0_f64;
    let mut total_max = 0.0_f64;
    for sq in session {
        let viewed_hint_indices = hint_usage
            .get(&sq.id)
            .into_iter()
            .flatten()
            .copied()
            .filter(|index| *index < sq.hints.len())
            .collect();
        if sq.kind == QuestionType::ShortAnswer {
            let response = match selections.get(&sq.id) {
                Some(Response::Text(text)) => text.clone(),
                _ => String::new(),
            };
            let status = if response.trim().is_empty() {
                QuestionStatus::UnansweredUnmarked
            } else {
                QuestionStatus::AnsweredUnmarked
            };
            results.push(QuestionResult {
                id: sq.id.clone(),
                text: sq.text.clone(),
                status,
                max_score: sq.marks,
                details: ResultDetails::ShortAnswer { response },
                metadata: sq.metadata.clone(),
                hints: sq.hints.clone(),
                viewed_hint_indices,
            });
            continue;
        }
        let selected = match selections.get(&sq.id) {
            Some(Response::Choices(ids)) => ids.clone(),
            _ => HashSet::new(),
        };
        let (score, max, status) = score_question(
            &sq.correct_ids,
            &selected,
            sq.options.len(),
            sq.marks,
            config.allow_negative_mark,
        );
        let correct_answers: Vec<(String, String)> = sq
            .options
            .iter()
            .filter(|(id, _, _)| sq.correct_ids.contains(id))
            .map(|(_, l, t)| (l.clone(), t.clone()))
            .collect();
        let selected_answers: Vec<(String, String)> = sq
            .options
            .iter()
            .filter(|(id, _, _)| selected.contains(id))
            .map(|(_, l, t)| (l.clone(), t.clone()))
            .collect();
        let missed_correct_answers: Vec<(String, String)> = sq
            .options
            .iter()
            .filter(|(id, _, _)| sq.correct_ids.contains(id) && !selected.contains(id))
            .map(|(_, l, t)| (l.clone(), t.clone()))
            .collect();
        let wrong_selected_answers: Vec<(String, String)> = sq
            .options
            .iter()
            .filter(|(id, _, _)| !sq.correct_ids.contains(id) && selected.contains(id))
            .map(|(_, l, t)| (l.clone(), t.clone()))
            .collect();
        total_score += score;
        total_max += max;
        results.push(QuestionResult {
            id: sq.id.clone(),
            text: sq.text.clone(),
            status,
            max_score: max,
            hints: sq.hints.clone(),
            viewed_hint_indices,
            details: ResultDetails::MultipleChoice(ChoiceResult {
                score,
                selected_answers,
                correct_answers,
                missed_correct_answers,
                wrong_selected_answers,
            }),
            metadata: sq.metadata.clone(),
        });
    }
    if !config.allow_negative_mark {
        total_score = total_score.max(0.0);
    }
    (results, total_score, total_max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn sample_quiz() -> QuizFile {
        QuizFile {
            title: "Test Quiz".into(),
            config: QuizConfig {
                marks_per_question: 2.0,
                allow_negative_mark: false,
                title: None,
                description: None,
                default_question_count: None,
            },
            marks_per_question: None,
            negative_marks: None,
            questions: vec![
                QuizQuestion {
                    kind: QuestionType::MultipleChoice,
                    marks: None,
                    hints: vec![],
                    id: "q1".into(),
                    question: "What is 2+2?".into(),
                    correct_answers: vec!["4".into()],
                    incorrect_answers: vec!["3".into(), "5".into()],
                    explanation: None,
                    metadata: QuestionMetadata {
                        study_location: Some("Chapter 1".into()),
                        topic: Some("Arithmetic".into()),
                        notes: None,
                        timestamp: None,
                        answer: None,
                        explanation: None,
                    },
                },
                QuizQuestion {
                    kind: QuestionType::MultipleChoice,
                    marks: None,
                    hints: vec![],
                    id: "q2".into(),
                    question: "Select all even numbers".into(),
                    correct_answers: vec!["2".into(), "4".into()],
                    incorrect_answers: vec!["3".into()],
                    explanation: Some("Even numbers are divisible by 2.".into()),
                    metadata: QuestionMetadata {
                        topic: Some("Number Theory".into()),
                        study_location: Some("Chapter 2".into()),
                        notes: Some("Watch out for trick questions with zero".into()),
                        timestamp: None,
                        answer: None,
                        explanation: None,
                    },
                },
                QuizQuestion {
                    kind: QuestionType::MultipleChoice,
                    marks: None,
                    hints: vec![],
                    id: "q3".into(),
                    question: "Capital of France?".into(),
                    correct_answers: vec!["Paris".into()],
                    incorrect_answers: vec!["London".into(), "Berlin".into(), "Madrid".into()],
                    explanation: None,
                    metadata: QuestionMetadata {
                        topic: Some("Geography".into()),
                        study_location: Some("Week 3, slide 7".into()),
                        timestamp: Some("Week 3, slide 7".into()),
                        answer: Some("Paris".into()),
                        notes: None,
                        explanation: None,
                    },
                },
            ],
        }
    }

    #[test]
    fn exports_completed_attempt_with_answers_metadata_source_and_seed() {
        let mut quiz = sample_quiz();
        quiz.title = "Quiz \"quoted\"\n日本語 </script>".into();
        let mut rng = SmallRng::from_seed([2; 32]);
        let session = build_session(&quiz.questions, 3, 2.0, &mut rng);
        let selections = session
            .iter()
            .map(|q| {
                (
                    q.id.clone(),
                    Response::Choices([q.options[0].0.clone()].into_iter().collect()),
                )
            })
            .collect();
        let results = build_results(
            &session,
            &selections,
            &HintUsage::new(),
            &quiz.effective_config(),
        );
        let text = results_json(&quiz, "input\\quiz.json", 42, &session, &results).unwrap();
        let exported: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(text.contains("\n  \"format_version\""));
        assert_eq!(exported["format_version"], 3);
        assert_eq!(exported["quiz_title"], quiz.title);
        assert_eq!(exported["source_file"], "input\\quiz.json");
        assert_eq!(exported["seed"], 42);
        assert_eq!(exported["question_count"], 3);
        assert_eq!(exported["total_score"], results.1);
        assert_eq!(exported["total_max"], results.2);
        assert_eq!(exported["percentage"], results.1 / results.2 * 100.0);
        assert_eq!(exported["config"]["marks_per_question"], 2.0);
        for (json, result) in exported["questions"]
            .as_array()
            .unwrap()
            .iter()
            .zip(&results.0)
        {
            assert_eq!(json["id"], result.id);
            let labels = |answers: &[(String, String)]| {
                answers
                    .iter()
                    .map(|(label, _)| label.to_ascii_lowercase())
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                json["selected"],
                serde_json::to_value(labels(&result.choice().unwrap().selected_answers)).unwrap(),
            );
            assert_eq!(
                json["correct"],
                serde_json::to_value(labels(&result.choice().unwrap().correct_answers)).unwrap(),
            );
            assert_eq!(
                json["missed"],
                serde_json::to_value(labels(&result.choice().unwrap().missed_correct_answers),)
                    .unwrap(),
            );
            assert_eq!(
                json["incorrect"],
                serde_json::to_value(labels(&result.choice().unwrap().wrong_selected_answers),)
                    .unwrap(),
            );
            assert_eq!(
                json["metadata"],
                serde_json::to_value(&result.metadata).unwrap(),
            );
        }
    }

    #[test]
    fn export_handles_unanswered_and_empty_results() {
        let quiz = sample_quiz();
        let mut rng = SmallRng::from_seed([0; 32]);
        let session = build_session(&quiz.questions, 1, 2.0, &mut rng);
        let results = build_results(
            &session,
            &Default::default(),
            &HintUsage::new(),
            &quiz.effective_config(),
        );
        let exported: serde_json::Value =
            serde_json::from_str(&results_json(&quiz, "quiz.json", 0, &session, &results).unwrap())
                .unwrap();
        assert_eq!(exported["seed"], 0);
        assert_eq!(exported["questions"][0]["status"], "unanswered");
        assert_eq!(exported["questions"][0]["selected"], serde_json::json!([]));
        assert_eq!(exported["percentage"], 0.0);
        let empty: serde_json::Value = serde_json::from_str(
            &results_json(&quiz, "", u32::MAX, &[], &(vec![], 0.0, 0.0)).unwrap(),
        )
        .unwrap();
        assert_eq!(empty["seed"], u32::MAX);
        assert_eq!(empty["percentage"], serde_json::Value::Null);
        assert_eq!(empty["questions"], serde_json::json!([]));
    }

    #[test]
    fn export_v3_retains_all_options_and_classifies_labels() {
        let quiz = sample_quiz();
        let session = vec![SessionQuestion {
            kind: QuestionType::MultipleChoice,
            marks: 2.0,
            hints: vec![],
            id: "q1".into(),
            text: "Which numbers are even?".into(),
            options: [
                ("opt_0", "A", "2"),
                ("opt_1", "B", "4"),
                ("opt_2", "C", "3"),
                ("opt_3", "D", "5"),
            ]
            .map(|(id, label, text)| (id.into(), label.into(), text.into()))
            .to_vec(),
            correct_ids: ["opt_0".into(), "opt_1".into()].into_iter().collect(),
            metadata: QuestionMetadata::default(),
        }];
        for (selected_ids, selected, missed, incorrect, status, score) in [
            (
                vec!["opt_0", "opt_2"],
                vec!["a", "c"],
                vec!["b"],
                vec!["c"],
                "partially_correct",
                0.0,
            ),
            (
                vec!["opt_0", "opt_1"],
                vec!["a", "b"],
                vec![],
                vec![],
                "correct",
                2.0,
            ),
            (vec![], vec![], vec!["a", "b"], vec![], "unanswered", 0.0),
            (
                vec!["opt_2"],
                vec!["c"],
                vec!["a", "b"],
                vec!["c"],
                "incorrect",
                0.0,
            ),
        ] {
            let selections = [(
                "q1".into(),
                Response::Choices(selected_ids.into_iter().map(String::from).collect()),
            )]
            .into_iter()
            .collect();
            let results = build_results(
                &session,
                &selections,
                &HintUsage::new(),
                &quiz.effective_config(),
            );
            let json: serde_json::Value = serde_json::from_str(
                &results_json(&quiz, "quiz.json", 42, &session, &results).unwrap(),
            )
            .unwrap();
            assert_eq!(
                json["questions"][0],
                serde_json::json!(
                    { "id" : "q1", "type" : "multiple_choice", "hints" : [],
                    "hints_viewed" : false, "viewed_hint_indices" : [], "text" :
                    "Which numbers are even?", "status" : status, "score" : score,
                    "max_score" : 2.0, "answers" : { "a" : "2", "b" : "4", "c" : "3", "d"
                    : "5" }, "correct" : ["a", "b"], "selected" : selected, "missed" :
                    missed, "incorrect" : incorrect, "metadata" :
                    serde_json::to_value(QuestionMetadata::default()).unwrap(), }
                ),
            );
            assert_eq!(json["total_score"], score);
            assert_eq!(json["total_max"], 2.0);
            assert_eq!(json["percentage"], score / 2.0 * 100.0);
        }
    }

    #[test]
    fn export_rejects_results_from_a_different_session() {
        let quiz = sample_quiz();
        let mut session = build_seeded_session(&quiz.questions, 3, 42, 2.0);
        let results = build_results(
            &session,
            &Default::default(),
            &HintUsage::new(),
            &quiz.effective_config(),
        );
        session.swap(0, 1);
        assert_eq!(
            results_json(&quiz, "quiz.json", 42, &session, &results)
                .unwrap_err()
                .to_string(),
            "Completed results do not match the session",
        );
        session.swap(0, 1);
        session.pop();
        assert!(results_json(&quiz, "quiz.json", 42, &session, &results).is_err());
    }

    #[test]
    fn seed_validation_boundaries() {
        for (input, expected) in [("0", 0), ("4294967295", u32::MAX), (" 00042\t", 42)] {
            assert_eq!(parse_seed(input), Ok(expected));
        }
        for input in [
            "",
            " ",
            "-1",
            "+1",
            "1.0",
            "1e3",
            "0x10",
            "1 2",
            "１２",
            "4294967296",
        ] {
            assert!(parse_seed(input).is_err(), "accepted {input:?}");
        }
    }

    #[test]
    fn seeded_fixture_order_and_correct_mappings() {
        let quiz: QuizFile =
            serde_json::from_str(include_str!("../tests/fixtures/seeded-quiz.json")).unwrap();
        let session = build_seeded_session(&quiz.questions, 4, 42, 2.0);
        let actual: Vec<_> = session
            .iter()
            .map(|q| {
                serde_json::json!(
                    { "id" : q.id, "text" : q.text, "options" : q.options.iter().map(|
                    (_, _, text) | text).collect::< Vec < _ >> (), }
                )
            })
            .collect();
        let expected: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../tests/fixtures/seeded-order.json")).unwrap();
        assert_eq!(actual, expected);
        for question in &session {
            let source = quiz.questions.iter().find(|q| q.id == question.id).unwrap();
            for (id, _, text) in &question.options {
                assert_eq!(
                    question.correct_ids.contains(id),
                    source.correct_answers.contains(text),
                );
            }
            if question.id == "two" {
                assert_eq!(question.options[0].2, "True");
                assert_eq!(question.options[1].2, "False");
                assert_eq!(question.correct_ids, HashSet::from(["opt_1".to_string()]));
            }
        }
        assert_eq!(session, build_seeded_session(&quiz.questions, 4, 42, 2.0));
        assert_eq!(
            session[..2],
            build_seeded_session(&quiz.questions, 2, 42, 2.0)
        );
        assert_ne!(session, build_seeded_session(&quiz.questions, 4, 43, 2.0));
        assert_eq!(session, build_seeded_session(&quiz.questions, 100, 42, 2.0));
        assert!(build_seeded_session(&[], 4, 42, 2.0).is_empty());
        assert!(build_seeded_session(&quiz.questions, 0, 42, 2.0).is_empty());
    }

    #[test]
    fn default_config_allows_no_negative() {
        assert!(!QuizConfig::default().allow_negative_mark);
        assert_eq!(QuizConfig::default().marks_per_question, 1.0);
    }

    #[test]
    fn parses_preferred_layout() {
        let json = r#"{
            "title":"T",
            "config":{"marks_per_question":2,"allow_negative_mark":false},
            "questions":[{"id":"q1","question":"Q?","correct_answers":["A"],"incorrect_answers":["B"],"metadata":{"study_location":"Week 1"}}]
        }"#;
        let quiz: QuizFile = serde_json::from_str(json).unwrap();
        assert_eq!(quiz.display_title(), "T");
        assert_eq!(quiz.effective_config().marks_per_question, 2.0);
        assert!(!quiz.effective_config().allow_negative_mark);
        assert_eq!(
            quiz.questions[0].metadata.study_location.as_deref(),
            Some("Week 1")
        );
    }

    #[test]
    fn parses_user_example_aliases() {
        let json = r#"{
            "marks_per_question":2,
            "negative_marks":false,
            "questions":[{
                "id":"MCQ-1",
                "question":"Q?",
                "correctAnswers":["A"],
                "incorrectAnswers":["B"],
                "hiddenInfo":{"timestamp":"Week 13, slide 4","answer":"B","explanation":"Read the slide."}
            }]
        }"#;
        let quiz: QuizFile = serde_json::from_str(json).unwrap();
        let config = quiz.effective_config();
        assert_eq!(config.marks_per_question, 2.0);
        assert!(!config.allow_negative_mark);
        assert_eq!(quiz.questions[0].correct_answers, vec!["A"]);
        assert_eq!(
            quiz.questions[0].metadata.timestamp.as_deref(),
            Some("Week 13, slide 4"),
        );
        assert_eq!(quiz.questions[0].metadata.answer.as_deref(), Some("B"));
    }

    #[test]
    fn missing_config_defaults_to_no_negative() {
        let json = r#"{"title":"T","questions":[{"id":"q1","question":"Q?","correct_answers":["A"],"incorrect_answers":["B"],"metadata":{}}]}"#;
        let quiz: QuizFile = serde_json::from_str(json).unwrap();
        assert!(!quiz.effective_config().allow_negative_mark);
        assert_eq!(quiz.effective_config().marks_per_question, 1.0);
    }

    #[test]
    fn validate_quiz_accepts_more_than_5_answers() {
        let mut quiz = sample_quiz();
        quiz.questions[0].correct_answers = vec!["a".into(), "b".into(), "c".into()];
        quiz.questions[0].incorrect_answers = vec!["d".into(), "e".into(), "f".into()];
        assert!(validate_quiz(&quiz).is_ok());
    }

    #[test]
    fn validate_quiz_rejects_no_correct_answers() {
        let mut quiz = sample_quiz();
        quiz.questions[0].correct_answers = vec![];
        let result = validate_quiz(&quiz);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no correct answers"));
    }

    #[test]
    fn validate_quiz_accepts_valid() {
        assert!(validate_quiz(&sample_quiz()).is_ok());
    }

    #[test]
    fn initial_count_uses_config_default_and_clamps() {
        let mut quiz = sample_quiz();
        quiz.config.default_question_count = Some(2);
        assert_eq!(initial_question_count(&quiz), 2);
        quiz.config.default_question_count = Some(100);
        assert_eq!(initial_question_count(&quiz), 3);
    }

    #[test]
    fn build_session_respects_count() {
        let mut rng = SmallRng::from_seed([0u8; 32]);
        assert_eq!(
            build_session(&sample_quiz().questions, 2, 2.0, &mut rng).len(),
            2
        );
    }

    #[test]
    fn build_session_clamps_count() {
        let mut rng = SmallRng::from_seed([0u8; 32]);
        assert_eq!(
            build_session(&sample_quiz().questions, 100, 2.0, &mut rng).len(),
            3
        );
    }

    #[test]
    fn build_session_labels_a_through_e() {
        let mut rng = SmallRng::from_seed([1u8; 32]);
        let sq = &build_session(&sample_quiz().questions, 1, 2.0, &mut rng)[0];
        for (_, label, _) in &sq.options {
            assert!(["A", "B", "C", "D", "E"].contains(&label.as_str()));
        }
    }

    #[test]
    fn score_correct_single() {
        let c: HashSet<String> = ["opt_0".into()].into_iter().collect();
        let s: HashSet<String> = ["opt_0".into()].into_iter().collect();
        let (score, max, status) = score_question(&c, &s, 2, 2.0, false);
        assert_eq!(score, 2.0);
        assert_eq!(max, 2.0);
        assert_eq!(status, QuestionStatus::Correct);
    }

    #[test]
    fn score_partial_correct() {
        let c: HashSet<String> = ["opt_0".into(), "opt_1".into()].into_iter().collect();
        let s: HashSet<String> = ["opt_0".into()].into_iter().collect();
        let (score, _, status) = score_question(&c, &s, 3, 2.0, false);
        assert!((score - 1.0).abs() < 0.001);
        assert_eq!(status, QuestionStatus::PartiallyCorrect);
    }

    #[test]
    fn score_unanswered() {
        let c: HashSet<String> = ["opt_0".into()].into_iter().collect();
        let s = HashSet::new();
        let (score, _, status) = score_question(&c, &s, 3, 2.0, false);
        assert_eq!(score, 0.0);
        assert_eq!(status, QuestionStatus::Unanswered);
    }

    #[test]
    fn score_wrong_clamped_zero() {
        let c: HashSet<String> = ["opt_0".into()].into_iter().collect();
        let s: HashSet<String> = ["opt_1".into()].into_iter().collect();
        let (score, _, status) = score_question(&c, &s, 2, 2.0, false);
        assert_eq!(score, 0.0);
        assert_eq!(status, QuestionStatus::Incorrect);
    }

    #[test]
    fn score_negative_allowed() {
        let c: HashSet<String> = ["opt_0".into(), "opt_1".into()].into_iter().collect();
        let s: HashSet<String> = ["opt_2".into()].into_iter().collect();
        let (score, _, _) = score_question(&c, &s, 3, 2.0, true);
        assert!(score < 0.0);
    }

    #[test]
    fn score_negative_disabled_clamps() {
        let c: HashSet<String> = ["opt_0".into(), "opt_1".into()].into_iter().collect();
        let s: HashSet<String> = ["opt_2".into()].into_iter().collect();
        let (score, _, _) = score_question(&c, &s, 3, 2.0, false);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn score_partial_with_wrong_penalty() {
        let c: HashSet<String> = ["opt_0".into(), "opt_1".into()].into_iter().collect();
        let s: HashSet<String> = ["opt_0".into(), "opt_2".into()].into_iter().collect();
        let (score, _, status) = score_question(&c, &s, 3, 2.0, true);
        assert!((score + 1.0).abs() < 0.001);
        assert_eq!(status, QuestionStatus::PartiallyCorrect);
    }

    #[test]
    fn results_clamped_without_negative() {
        let mut rng = SmallRng::from_seed([42u8; 32]);
        let quiz = sample_quiz();
        let session = build_session(&quiz.questions, 3, 2.0, &mut rng);
        let (results, total, max) = build_results(
            &session,
            &std::collections::HashMap::new(),
            &HintUsage::new(),
            &quiz.effective_config(),
        );
        assert_eq!(results.len(), 3);
        assert!(total >= 0.0);
        assert_eq!(max, 6.0);
    }

    #[test]
    fn example_quiz_json_parses_and_validates() {
        let json = EXAMPLE_QUIZ_JSON.replace("SCHEMA_URL", "");
        let quiz: QuizFile =
            serde_json::from_str(&json).expect("EXAMPLE_QUIZ_JSON should parse as QuizFile");
        validate_quiz(&quiz).expect("EXAMPLE_QUIZ_JSON should pass validate_quiz");
    }

    #[test]
    fn results_negative_allowed() {
        let mut rng = SmallRng::from_seed([42u8; 32]);
        let mut quiz = sample_quiz();
        quiz.config.allow_negative_mark = true;
        let session = build_session(&quiz.questions, 3, 2.0, &mut rng);
        let mut selections = std::collections::HashMap::new();
        for sq in &session {
            let wrong: HashSet<String> = sq
                .options
                .iter()
                .filter(|(id, _, _)| !sq.correct_ids.contains(id))
                .map(|(id, _, _)| id.clone())
                .collect();
            if !wrong.is_empty() {
                selections.insert(sq.id.clone(), Response::Choices(wrong));
            }
        }
        let (_, total, _) = build_results(
            &session,
            &selections,
            &HintUsage::new(),
            &quiz.effective_config(),
        );
        assert!(total < 0.0);
    }
    fn mixed_quiz() -> QuizFile {
        serde_json::from_value(serde_json::json!(
            { "marks_per_question" : 2, "config" : { "marks_per_question" : 9 },
            "questions" : [{ "id" : "mc", "question" : "Choose", "marks" : 1.25,
            "correct_answers" : ["A", "B"], "incorrect_answers" : ["C", "D"],
            "hints" : ["First", "Second"] }, { "id" : "sa", "type" :
            "short_answer", "question" : "Explain", "marks" : 5.5, "hints" :
            ["Why?", "How?"], "metadata" : { "topic" : "Reasoning" } }, { "id" :
            "blank", "type" : "short_answer", "question" : "Optional" }] }
        ))
        .unwrap()
    }
    #[test]
    fn resolves_marks_precedence_and_keeps_fractional_marks_through_reruns() {
        let mut quiz = mixed_quiz();
        for (top_level, configured, expected_default) in
            [(Some(2.0), 9.0, 2.0), (None, 9.0, 9.0), (None, 1.0, 1.0)]
        {
            quiz.marks_per_question = top_level;
            quiz.config.marks_per_question = configured;
            let mut session = build_seeded_session(
                &quiz.questions,
                3,
                42,
                quiz.effective_config().marks_per_question,
            );
            let marks = |session: &[SessionQuestion]| {
                session
                    .iter()
                    .map(|q| (q.id.clone(), q.marks))
                    .collect::<BTreeMap<_, _>>()
            };
            let original = marks(&session);
            assert_eq!(original["mc"], 1.25);
            assert_eq!(original["sa"], 5.5);
            assert_eq!(original["blank"], expected_default);
            shuffle_session(&mut session, &mut SmallRng::seed_from_u64(7));
            assert_eq!(marks(&session), original);
        }
    }
    #[test]
    fn mixed_results_only_score_choices_and_preserve_exact_text() {
        let quiz = mixed_quiz();
        let session = build_seeded_session(
            &quiz.questions,
            3,
            42,
            quiz.effective_config().marks_per_question,
        );
        let mc = session.iter().find(|q| q.id == "mc").unwrap();
        let one_correct = mc.correct_ids.iter().next().unwrap().clone();
        let raw = "  日本語 🦀\n<script>alert('literal')</script>\n\t ";
        let responses = HashMap::from([
            ("mc".into(), Response::Choices(HashSet::from([one_correct]))),
            ("sa".into(), Response::Text(raw.into())),
            ("blank".into(), Response::Text(" \n\t".into())),
        ]);
        let results = build_results(
            &session,
            &responses,
            &HintUsage::new(),
            &quiz.effective_config(),
        );
        assert_eq!((results.1, results.2), (0.625, 1.25));
        let json: serde_json::Value = serde_json::from_str(
            &results_json(&quiz, "mixed.json", 42, &session, &results).unwrap(),
        )
        .unwrap();
        assert_eq!(json["format_version"], 3);
        assert_eq!(json["total_available_marks"], 8.75);
        assert_eq!(json["scored_question_count"], 1);
        assert_eq!(json["question_count"], 3);
        assert_eq!(json["percentage"], 50.0);
        let questions = json["questions"].as_array().unwrap();
        let sa = questions.iter().find(|q| q["id"] == "sa").unwrap();
        assert_eq!(sa["response"], raw);
        assert_eq!(sa["status"], "answered_unmarked");
        assert_eq!(sa["max_score"], 5.5);
        for field in [
            "score",
            "answers",
            "selected",
            "correct",
            "missed",
            "incorrect",
        ] {
            assert!(sa.get(field).is_none(), "unexpected {field}");
        }
        let blank = questions.iter().find(|q| q["id"] == "blank").unwrap();
        assert_eq!(blank["response"], " \n\t");
        assert_eq!(blank["status"], "unanswered_unmarked");
    }
    #[test]
    fn short_answer_only_has_no_percentage_or_awarded_scores() {
        let mut quiz = mixed_quiz();
        quiz.questions
            .retain(|q| q.kind == QuestionType::ShortAnswer);
        let session = build_seeded_session(&quiz.questions, 2, 0, 2.0);
        let results = build_results(
            &session,
            &Responses::new(),
            &HintUsage::new(),
            &quiz.effective_config(),
        );
        assert_eq!((results.1, results.2), (0.0, 0.0));
        let json: serde_json::Value =
            serde_json::from_str(&results_json(&quiz, "", 0, &session, &results).unwrap()).unwrap();
        assert!(json["percentage"].is_null());
        assert_eq!(json["total_available_marks"], 7.5);
        assert_eq!(json["scored_question_count"], 0);
        assert!(
            json["questions"]
                .as_array()
                .unwrap()
                .iter()
                .all(|q| q.get("score").is_none() && q["response"] == ""),
        );
    }
    #[test]
    fn hint_history_filters_stale_indices_and_snapshots_both_types() {
        let quiz = mixed_quiz();
        let session = build_seeded_session(&quiz.questions, 3, 42, 2.0);
        let mut usage = HintUsage::new();
        record_hint_view(&session, &mut usage, "missing", 0);
        record_hint_view(&session, &mut usage, "blank", 0);
        record_hint_view(&session, &mut usage, "mc", 2);
        assert!(usage.is_empty());
        for id in ["mc", "sa"] {
            record_hint_view(&session, &mut usage, id, 1);
            record_hint_view(&session, &mut usage, id, 1);
            record_hint_view(&session, &mut usage, id, 0);
            usage.get_mut(id).unwrap().insert(999);
        }
        usage.insert("blank".into(), BTreeSet::from([0, 1, 999]));
        let results = build_results(
            &session,
            &Responses::new(),
            &usage,
            &quiz.effective_config(),
        );
        usage.clear();
        let json: serde_json::Value =
            serde_json::from_str(&results_json(&quiz, "", 42, &session, &results).unwrap())
                .unwrap();
        for q in json["questions"].as_array().unwrap() {
            if q["id"] == "blank" {
                assert_eq!(q["hints"], serde_json::json!([]));
                assert_eq!(q["hints_viewed"], false);
                assert_eq!(q["viewed_hint_indices"], serde_json::json!([]));
            } else {
                assert_eq!(q["hints_viewed"], true);
                assert_eq!(q["viewed_hint_indices"], serde_json::json!([0, 1]));
            }
        }
        assert_eq!(results.1, 0.0);
        assert_eq!(
            results.0.iter().find(|q| q.id == "mc").unwrap().status,
            QuestionStatus::Unanswered,
        );
    }
    #[test]
    fn omitted_and_empty_hints_ignore_stale_usage_on_both_types() {
        for kind in [QuestionType::MultipleChoice, QuestionType::ShortAnswer] {
            for explicit_empty in [false, true] {
                let mut q = serde_json::json!(
                    { "id" : "q", "type" : kind, "question" : "Q", "correct_answers" :
                    [], "incorrect_answers" : [] }
                );
                if kind == QuestionType::MultipleChoice {
                    q["correct_answers"] = serde_json::json!(["A"]);
                }
                if explicit_empty {
                    q["hints"] = serde_json::json!([]);
                }
                let quiz: QuizFile =
                    serde_json::from_value(serde_json::json!({ "questions" : [q] })).unwrap();
                validate_quiz(&quiz).unwrap();
                let session = build_seeded_session(&quiz.questions, 1, 0, 1.0);
                let usage = HashMap::from([("q".into(), BTreeSet::from([0, 99]))]);
                let results = build_results(
                    &session,
                    &Responses::new(),
                    &usage,
                    &quiz.effective_config(),
                );
                assert!(results.0[0].viewed_hint_indices.is_empty());
                let json: serde_json::Value =
                    serde_json::from_str(&results_json(&quiz, "", 0, &session, &results).unwrap())
                        .unwrap();
                assert_eq!(json["questions"][0]["hints_viewed"], false);
                assert_eq!(
                    json["questions"][0]["viewed_hint_indices"],
                    serde_json::json!([]),
                );
            }
        }
    }
    #[test]
    fn rejects_invalid_marks_types_values_and_nonfinite_values() {
        for kind in ["multiple_choice", "short_answer"] {
            for marks in [
                serde_json::json!(0),
                serde_json::json!(-1),
                serde_json::json!(null),
                serde_json::json!("2"),
                serde_json::json!(true),
            ] {
                let mut q = serde_json::json!(
                    { "id" : "q", "type" : kind, "question" : "Q", "marks" : marks }
                );
                if kind == "multiple_choice" {
                    q["correct_answers"] = serde_json::json!(["A"]);
                    q["incorrect_answers"] = serde_json::json!([]);
                }
                let input = serde_json::json!({ "questions" : [q] });
                assert!(
                    serde_json::from_value::<QuizFile>(input.clone())
                        .map(|q| validate_quiz(&q).is_err())
                        .unwrap_or(true),
                    "accepted {input}",
                );
            }
        }
        for marks in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut quiz = mixed_quiz();
            quiz.questions[0].marks = Some(marks);
            assert!(validate_quiz(&quiz).unwrap_err().contains("finite"));
            quiz.questions[0].marks = None;
            quiz.marks_per_question = Some(marks);
            assert!(validate_quiz(&quiz).is_err());
        }
    }
    #[test]
    fn rejects_invalid_hint_arrays_for_both_types() {
        for kind in ["multiple_choice", "short_answer"] {
            for hints in [
                serde_json::json!(null),
                serde_json::json!("hint"),
                serde_json::json!([1]),
                serde_json::json!([""]),
                serde_json::json!([" \n\t"]),
            ] {
                let mut q = serde_json::json!(
                    { "id" : "q", "type" : kind, "question" : "Q", "hints" : hints }
                );
                if kind == "multiple_choice" {
                    q["correct_answers"] = serde_json::json!(["A"]);
                    q["incorrect_answers"] = serde_json::json!([]);
                }
                assert!(
                    serde_json::from_value::<QuizFile>(serde_json::json!({ "questions" : [q] }))
                        .map(|q| validate_quiz(&q).is_err())
                        .unwrap_or(true),
                );
            }
        }
    }
    #[test]
    fn rejects_unknown_types_duplicate_ids_and_short_answer_options() {
        assert!(
            serde_json::from_str::<QuizFile>(
                r#"{"questions":[{"id":"q","type":"essay","question":"Q"}]}"#
            )
            .unwrap_err()
            .to_string()
            .contains("unknown variant"),
        );
        let mut quiz = mixed_quiz();
        quiz.questions[1].id = quiz.questions[0].id.clone();
        assert!(validate_quiz(&quiz).unwrap_err().contains("Duplicate"));
        let mut quiz = mixed_quiz();
        quiz.questions[1]
            .incorrect_answers
            .push("not allowed".into());
        assert!(
            validate_quiz(&quiz)
                .unwrap_err()
                .contains("must not have answer options"),
        );
        for input in [
            r#"{"id":"q","question":"Q","correct_answers":["A"]}"#,
            r#"{"id":"q","question":"Q","incorrect_answers":[]}"#,
        ] {
            assert!(
                serde_json::from_str::<QuizQuestion>(input)
                    .unwrap_err()
                    .to_string()
                    .contains("requires"),
            );
        }
    }
    #[test]
    fn response_completion_and_wrong_response_variants_are_safe() {
        for text in ["", " ", "\n\t", "\u{2003}"] {
            assert!(!Response::Text(text.into()).is_answered());
        }
        assert!(Response::Text(" 🦀\n ".into()).is_answered());
        assert!(!Response::Choices(HashSet::new()).is_answered());
        let quiz = mixed_quiz();
        let session = build_seeded_session(&quiz.questions, 3, 0, 2.0);
        let responses = HashMap::from([
            ("mc".into(), Response::Text("text".into())),
            (
                "sa".into(),
                Response::Choices(HashSet::from(["opt_0".into()])),
            ),
        ]);
        let result = build_results(
            &session,
            &responses,
            &HintUsage::new(),
            &quiz.effective_config(),
        );
        assert_eq!(result.1, 0.0);
        assert!(result.0.iter().all(|q| {
            matches!(
                q.status,
                QuestionStatus::Unanswered | QuestionStatus::UnansweredUnmarked
            )
        }),);
    }
    #[test]
    fn sums_weighted_mcq_maxima_separately_from_all_available_marks() {
        let mut quiz = mixed_quiz();
        let mut other = quiz.questions[0].clone();
        other.id = "other".into();
        other.marks = Some(2.75);
        quiz.questions.push(other);
        let session = build_seeded_session(&quiz.questions, 4, 42, 2.0);
        let mut responses = Responses::new();
        for q in &session {
            if q.kind == QuestionType::MultipleChoice {
                let choices = if q.id == "mc" {
                    q.correct_ids.iter().take(1).cloned().collect()
                } else {
                    q.correct_ids.clone()
                };
                responses.insert(q.id.clone(), Response::Choices(choices));
            }
        }
        let results = build_results(
            &session,
            &responses,
            &HintUsage::new(),
            &quiz.effective_config(),
        );
        assert_eq!((results.1, results.2), (3.375, 4.0));
        let json: serde_json::Value =
            serde_json::from_str(&results_json(&quiz, "", 42, &session, &results).unwrap())
                .unwrap();
        assert_eq!(json["total_available_marks"], 11.5);
        assert_eq!(json["percentage"], 84.375);
        assert_eq!(json["scored_question_count"], 2);
    }
}

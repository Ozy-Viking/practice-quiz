use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct QuizQuestion {
    pub id: String,
    pub question: String,
    #[serde(alias = "correctAnswers")]
    pub correct_answers: Vec<String>,
    #[serde(alias = "incorrectAnswers")]
    pub incorrect_answers: Vec<String>,
    #[serde(default)]
    pub explanation: Option<String>,
    #[serde(default, alias = "hiddenInfo")]
    pub metadata: QuestionMetadata,
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
    pub options: Vec<(String, String, String)>,
    pub correct_ids: HashSet<String>,
    pub metadata: QuestionMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionStatus {
    Correct,
    PartiallyCorrect,
    Incorrect,
    Unanswered,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuestionResult {
    pub id: String,
    pub text: String,
    pub status: QuestionStatus,
    pub score: f64,
    pub max_score: f64,
    pub selected_answers: Vec<(String, String)>,
    pub correct_answers: Vec<(String, String)>,
    pub missed_correct_answers: Vec<(String, String)>,
    pub wrong_selected_answers: Vec<(String, String)>,
    pub metadata: QuestionMetadata,
}

pub type ResultsData = (Vec<QuestionResult>, f64, f64);

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
      "metadata": {
        "topic": "Chapter 1",
        "study_location": "Week 1",
        "timestamp": "slide 4",
        "explanation": "Because..."
      }
    }
  ]
}"#;

pub fn validate_quiz(quiz: &QuizFile) -> Result<(), String> {
    let config = quiz.effective_config();
    if config.marks_per_question <= 0.0 {
        return Err("marks_per_question must be greater than 0".to_string());
    }
    if quiz.questions.is_empty() {
        return Err("Quiz file contains no questions".to_string());
    }
    for q in &quiz.questions {
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
                q.question, q.id
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

pub fn build_session(
    questions: &[QuizQuestion],
    count: usize,
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
            let labels: Vec<String> = (b'A'..=b'Z').map(|c| (c as char).to_string()).collect();
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
                .map(|(i, (_, text))| (format!("opt_{i}"), labels[i].to_string(), text.clone()))
                .collect();
            SessionQuestion {
                id: q.id,
                text: q.question,
                options,
                correct_ids,
                metadata,
            }
        })
        .collect()
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
    selections: &std::collections::HashMap<String, HashSet<String>>,
    config: &QuizConfig,
) -> ResultsData {
    let mut results = Vec::new();
    let mut total_score = 0.0_f64;
    let mut total_max = 0.0_f64;
    for sq in session {
        let selected = selections.get(&sq.id).cloned().unwrap_or_default();
        let (score, max, status) = score_question(
            &sq.correct_ids,
            &selected,
            sq.options.len(),
            config.marks_per_question,
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
            score,
            max_score: max,
            selected_answers,
            correct_answers,
            missed_correct_answers,
            wrong_selected_answers,
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
            Some("Week 13, slide 4")
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
            build_session(&sample_quiz().questions, 2, &mut rng).len(),
            2
        );
    }

    #[test]
    fn build_session_clamps_count() {
        let mut rng = SmallRng::from_seed([0u8; 32]);
        assert_eq!(
            build_session(&sample_quiz().questions, 100, &mut rng).len(),
            3
        );
    }

    #[test]
    fn build_session_labels_a_through_e() {
        let mut rng = SmallRng::from_seed([1u8; 32]);
        let sq = &build_session(&sample_quiz().questions, 1, &mut rng)[0];
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
        let session = build_session(&quiz.questions, 3, &mut rng);
        let (results, total, max) = build_results(
            &session,
            &std::collections::HashMap::new(),
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
        let session = build_session(&quiz.questions, 3, &mut rng);
        let mut selections = std::collections::HashMap::new();
        for sq in &session {
            let wrong: HashSet<String> = sq
                .options
                .iter()
                .filter(|(id, _, _)| !sq.correct_ids.contains(id))
                .map(|(id, _, _)| id.clone())
                .collect();
            if !wrong.is_empty() {
                selections.insert(sq.id.clone(), wrong);
            }
        }
        let (_, total, _) = build_results(&session, &selections, &quiz.effective_config());
        assert!(total < 0.0);
    }
}

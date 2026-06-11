mod components;
mod model;

use components::{ConfiguringView, QuizView, ResultsView, UploadView};
use dioxus::prelude::*;
use model::*;
use std::collections::{HashMap, HashSet};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let phase: Signal<AppPhase> = use_signal(|| AppPhase::Upload);
    let quiz: Signal<Option<QuizFile>> = use_signal(|| None);
    let session: Signal<Vec<SessionQuestion>> = use_signal(Vec::new);
    let selections: Signal<HashMap<String, HashSet<String>>> = use_signal(HashMap::new);
    let question_count: Signal<usize> = use_signal(|| 0);
    let file_name: Signal<String> = use_signal(String::new);
    let load_error: Signal<String> = use_signal(String::new);
    let results: Signal<Option<ResultsData>> = use_signal(|| None);
    let question_pool: Signal<Vec<QuizQuestion>> = use_signal(Vec::new);

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

mod components;
mod import;
mod model;

use components::{ConfiguringView, QuizView, ResultsView, UploadView};
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::ld_icons::{LdMoon, LdSun};
use model::*;
use std::collections::{HashMap, HashSet};

fn main() {
    dioxus::launch(App);
}

#[cfg(any(target_arch = "wasm32", test))]
fn resolve_light_mode(saved_theme: Option<&str>, system_prefers_light: bool) -> bool {
    match saved_theme {
        Some("light") => true,
        Some("dark") => false,
        _ => system_prefers_light,
    }
}

fn initial_light_mode() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            let saved_theme = window
                .local_storage()
                .ok()
                .flatten()
                .and_then(|storage| storage.get_item("practice-quiz-theme").ok().flatten());

            let system_prefers_light = window
                .match_media("(prefers-color-scheme: light)")
                .ok()
                .flatten()
                .is_some_and(|media| media.matches());

            return resolve_light_mode(saved_theme.as_deref(), system_prefers_light);
        }
    }

    false
}

fn save_light_mode(light: bool) {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item("practice-quiz-theme", if light { "light" } else { "dark" });
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = light;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_defaults_to_system_without_saved_preference() {
        assert!(resolve_light_mode(None, true));
        assert!(!resolve_light_mode(None, false));
    }

    #[test]
    fn saved_theme_overrides_system_default() {
        assert!(resolve_light_mode(Some("light"), false));
        assert!(!resolve_light_mode(Some("dark"), true));
    }

    #[test]
    fn invalid_saved_theme_falls_back_to_system() {
        assert!(resolve_light_mode(Some("system"), true));
        assert!(!resolve_light_mode(Some("unexpected"), false));
    }
}

#[cfg(target_arch = "wasm32")]
struct FileDropGuard {
    window: web_sys::Window,
    listener: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::DragEvent)>,
}

#[cfg(target_arch = "wasm32")]
impl FileDropGuard {
    fn install() -> Option<Self> {
        use wasm_bindgen::JsCast;
        let window = web_sys::window()?;
        let listener = wasm_bindgen::closure::Closure::new(|event: web_sys::DragEvent| {
            if event.data_transfer().is_some_and(|data| {
                data.types()
                    .iter()
                    .any(|kind| kind.as_string().as_deref() == Some("Files"))
            }) {
                event.prevent_default();
            }
        });
        let guard = Self { window, listener };
        for kind in ["dragover", "drop"] {
            guard
                .window
                .add_event_listener_with_callback(kind, guard.listener.as_ref().unchecked_ref())
                .ok()?;
        }
        Some(guard)
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for FileDropGuard {
    fn drop(&mut self) {
        use wasm_bindgen::JsCast;
        for kind in ["dragover", "drop"] {
            let _ = self
                .window
                .remove_event_listener_with_callback(kind, self.listener.as_ref().unchecked_ref());
        }
    }
}

#[component]
fn App() -> Element {
    // The hook owns the listeners until the app unmounts, across every quiz phase.
    #[cfg(target_arch = "wasm32")]
    use_hook(|| std::rc::Rc::new(FileDropGuard::install()));
    use_context_provider(|| Signal::new(std::rc::Rc::new(import::AssetStore::default())));
    let phase: Signal<AppPhase> = use_signal(|| AppPhase::Upload);
    let quiz: Signal<Option<QuizFile>> = use_signal(|| None);
    let session: Signal<Vec<SessionQuestion>> = use_signal(Vec::new);
    let selections: Signal<Responses> = use_signal(HashMap::new);
    let hint_usage: Signal<HintUsage> = use_signal(HashMap::new);
    let question_count: Signal<usize> = use_signal(|| 0);
    let file_name: Signal<String> = use_signal(String::new);
    let load_error: Signal<String> = use_signal(String::new);
    let results: Signal<Option<ResultsData>> = use_signal(|| None);
    let question_pool: Signal<Vec<QuizQuestion>> = use_signal(Vec::new);
    let selected_topics = use_signal(HashSet::<String>::new);
    let selected_locations = use_signal(HashSet::<String>::new);
    let topic_regex_input = use_signal(String::new);
    let location_regex_input = use_signal(String::new);
    let seed_input = use_signal(String::new);
    let active_seed = use_signal(|| 0u32);
    let mut light_mode = use_signal(initial_light_mode);

    let current = phase.read().clone();
    let is_light = *light_mode.read();
    let app_class = if is_light {
        "theme-light"
    } else {
        "theme-dark"
    };
    let theme_action = if is_light {
        "Switch to dark mode"
    } else {
        "Switch to light mode"
    };

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        div { id: "app", class: "{app_class}",
            header { class: "app-header",
                button {
                    class: if is_light { "theme-switch is-light" } else { "theme-switch" },
                    r#type: "button",
                    role: "switch",
                    aria_checked: "{is_light}",
                    aria_label: "{theme_action}",
                    onclick: move |_| {
                        let next = {
                            let current = light_mode.read();
                            !*current
                        };
                        light_mode.set(next);
                        save_light_mode(next);
                    },
                    span { class: "theme-switch-track",
                        Icon {
                            class: "theme-switch-icon theme-switch-moon",
                            width: 14,
                            height: 14,
                            title: Some("Dark mode".to_string()),
                            icon: LdMoon,
                        }
                        Icon {
                            class: "theme-switch-icon theme-switch-sun",
                            width: 14,
                            height: 14,
                            title: Some("Light mode".to_string()),
                            icon: LdSun,
                        }
                        span { class: "theme-switch-thumb" }
                    }
                }
                h1 { "Practice Quiz" }
                p { class: "subtitle", "Load a JSON or ZIP quiz and test your knowledge" }
            }
            if current == AppPhase::Upload {
                UploadView {
                    phase,
                    quiz,
                    question_count,
                    file_name,
                    load_error,
                    session,
                    selections,
                    hint_usage,
                    results,
                    question_pool,
                    selected_topics,
                    selected_locations,
                    topic_regex_input,
                    location_regex_input,
                    seed_input,
                    active_seed,
                }
            } else if current == AppPhase::Configuring {
                ConfiguringView {
                    phase,
                    quiz,
                    session,
                    selections,
                    hint_usage,
                    question_count,
                    file_name,
                    load_error,
                    question_pool,
                    selected_topics,
                    selected_locations,
                    topic_regex_input,
                    location_regex_input,
                    seed_input,
                    active_seed,
                    results,
                }
            } else if current == AppPhase::InProgress {
                QuizView {
                    phase,
                    quiz,
                    session,
                    selections,
                    hint_usage,
                    results,
                    active_seed,
                }
            } else if current == AppPhase::Submitted {
                ResultsView {
                    phase,
                    quiz,
                    session,
                    selections,
                    hint_usage,
                    results,
                    question_count,
                    file_name,
                    load_error,
                    question_pool,
                    active_seed,
                }
            }
        }
    }
}

use dioxus::prelude::*;
use std::collections::HashSet;

#[component]
pub fn FilterSection(
    title: String,
    mut selected: Signal<HashSet<String>>,
    mut regex_input: Signal<String>,
    all_values: Vec<String>,
    matched_chips: HashSet<String>,
    regex_active: bool,
    regex_valid: bool,
    placeholder: String,
) -> Element {
    let all_for_select = all_values.clone();
    rsx! {
        div { class: "filter-section",
            div { class: "filter-section-header",
                span { class: "filter-section-title", "{title}" }
                button {
                    class: "filter-action",
                    r#type: "button",
                    onclick: move |_| {
                        selected.set(all_for_select.iter().cloned().collect());
                    },
                    "All"
                }
                button {
                    class: "filter-action",
                    r#type: "button",
                    onclick: move |_| selected.set(HashSet::new()),
                    "None"
                }
            }
            div { class: "filter-regex-wrapper",
                input {
                    class: if regex_valid { "filter-regex-input" } else { "filter-regex-input invalid" },
                    r#type: "text",
                    placeholder: "{placeholder}",
                    value: "{regex_input.read()}",
                    oninput: move |evt| regex_input.set(evt.value()),
                }
                if !regex_valid {
                    span { class: "filter-regex-error", "Invalid regex" }
                }
            }
            div { class: if regex_active { "filter-chips regex-active" } else { "filter-chips" },
                for value in all_values.iter() {
                    {
                        let v = value.clone();
                        let is_checked = selected.read().contains(&v);
                        let chip_class = if regex_active {
                            if matched_chips.contains(&v) {
                                "filter-chip regex-match"
                            } else {
                                "filter-chip regex-no-match"
                            }
                        } else {
                            "filter-chip"
                        };
                        rsx! {
                            label { class: "{chip_class}", key: "{v}",
                                input {
                                    r#type: "checkbox",
                                    checked: is_checked,
                                    onchange: move |_| {
                                        let mut set = selected.read().clone();
                                        if set.contains(&v) {
                                            set.remove(&v);
                                        } else {
                                            set.insert(v.clone());
                                        }
                                        selected.set(set);
                                    },
                                }
                                "{v}"
                            }
                        }
                    }
                }
            }
        }
    }
}

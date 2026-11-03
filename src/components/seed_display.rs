use dioxus::prelude::*;

#[component]
pub fn SeedDisplay(seed: u32) -> Element {
    let mut copy_status = use_signal(String::new);
    rsx! {
        div { class: "seed-display",
            button {
                class: "session-seed seed-copy",
                r#type: "button",
                title: "Copy seed",
                aria_label: "Copy seed {seed}",
                onclick: move |_| async move {
                    copy_status.set(String::new());
                    #[cfg(target_arch = "wasm32")]
                    let copied = if let Some(window) = web_sys::window() {
                        wasm_bindgen_futures::JsFuture::from(
                            window.navigator().clipboard().write_text(&seed.to_string())
                        ).await.is_ok()
                    } else {
                        false
                    };
                    #[cfg(not(target_arch = "wasm32"))]
                    let copied = false;
                    copy_status.set(if copied { "Seed copied" } else { "Couldn't copy seed. Select and copy the number." }.to_string());
                },
                "Seed: {seed}"
            }
            span { class: "seed-copy-status", role: "status", "{copy_status}" }
        }
    }
}

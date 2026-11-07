use crate::{import::AssetStore, model::QuizImage};
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn QuestionImages(images: Vec<QuizImage>) -> Element {
    rsx! {
        if !images.is_empty() {
            div { class: "question-images",
                for (index, image) in images.into_iter().enumerate() {
                    Figure { key: "{index}:{image.src}", image }
                }
            }
        }
    }
}

#[component]
fn Figure(image: QuizImage) -> Element {
    let assets = use_context::<Signal<Rc<AssetStore>>>();
    let src = assets
        .read()
        .urls
        .get(&image.src)
        .cloned()
        .unwrap_or_else(|| image.src.clone());
    let mut failed = use_signal(|| false);
    let dialog_id = use_hook(|| format!("figure-{}", dioxus::core::current_scope_id().0));
    let open_id = dialog_id.clone();
    rsx! {
        figure { class: "question-figure",
            if failed() {
                p { role: "status", "Image could not load: {image.alt}" }
                button { r#type: "button", onclick: move |_| failed.set(false), "Retry image" }
            } else {
                button { class: "image-enlarge", r#type: "button", aria_label: "Enlarge image: {image.alt}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(dialog) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id(&open_id)).and_then(|e| e.dyn_into::<web_sys::HtmlDialogElement>().ok()) {
                                let _ = dialog.show_modal();
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &open_id;
                    },
                    img { src: "{src}", alt: "{image.alt}", referrerpolicy: "no-referrer", onerror: move |_| failed.set(true) }
                }
            }
            if let Some(caption) = &image.caption { figcaption { "{caption}" } }
            if !failed() {
                dialog { id: "{dialog_id}", class: "image-dialog", aria_label: "Enlarged image: {image.alt}",
                    form { method: "dialog", button { autofocus: true, "Close image" } }
                    img { src: "{src}", alt: "{image.alt}", referrerpolicy: "no-referrer", onerror: move |_| failed.set(true) }
                    if let Some(caption) = &image.caption { p { "{caption}" } }
                }
            }
        }
    }
}

use dioxus::prelude::*;
use stayhydated_dioxus::select;

#[component]
pub(crate) fn LanguageSelect<T: Clone + PartialEq + 'static>(
    label: String,
    selected: T,
    options: Vec<(T, String)>,
    on_change: EventHandler<T>,
) -> Element {
    let selected_label = options
        .iter()
        .find(|(option, _)| option == &selected)
        .map(|(_, label)| label.clone())
        .unwrap_or_default();
    let selected_value = ReadSignal::new(Signal::new(Some(selected.clone())));

    rsx! {
        div { class: "locale-switcher-dropdown",
            span { class: "locale-label", "{label}" }
            select::Select::<T> {
                value: Some(selected_value),
                on_value_change: move |next_locale: Option<T>| {
                    if let Some(next_locale) = next_locale {
                        on_change.call(next_locale);
                    }
                },
                select::SelectTrigger {
                    aria_label: label.clone(),
                    span { class: "header-locale-value", "{selected_label}" }
                }
                select::SelectList {
                    aria_label: label.clone(),
                    for (index, (option, option_label)) in options.iter().enumerate() {
                        {
                            let active = option == &selected;
                            let option_class = if active {
                                "header-locale-option is-active".to_string()
                            } else {
                                "header-locale-option".to_string()
                            };

                            rsx! {
                                select::SelectOption::<T> {
                                    index,
                                    value: option.clone(),
                                    text_value: Some(option_label.clone()),
                                    class: Some(option_class),
                                    span { "{option_label}" }
                                    if active {
                                        select::SelectItemIndicator {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

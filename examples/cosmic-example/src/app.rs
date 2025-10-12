use crate::config::Config;
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry as _};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::prelude::*;
use cosmic::widget::{self, icon, menu, nav_bar};
use cosmic::{cosmic_theme, theme};
use es_fluent::{EsFluent, ToFluentString as _};
use futures_util::SinkExt as _;
use std::collections::HashMap;
use strum::IntoEnumIterator as _;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    // Configuration data that persists between application runs.
    config: Config,
    current_language: example_shared_lib::Languages,
    current_page: Page,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    LaunchUrl(String),
    ToggleLanguage,
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.github.pop-os.cosmic-app-template";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Create a nav bar with three page items.
        let nav = nav_bar::Model::default();

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => {
                        // for why in errors {
                        //     tracing::error!(%why, "error loading app config");
                        // }

                        config
                    },
                })
                .unwrap_or_default(),
            current_language: example_shared_lib::Languages::English,
            current_page: Page::Page1,
        };

        app.rebuild_nav();
        let command = app.update_title();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(AppItems::View.to_fluent_string()).apply(Element::from),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button(
                    AppItems::About.to_fluent_string(),
                    None,
                    MenuAction::About,
                )],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::context_drawer(
                self.about(),
                Message::ToggleContextPage(ContextPage::About),
            )
            .title(AppItems::About.to_fluent_string()),
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        let title = widget::text::title1(AppItems::Welcome.to_fluent_string());

        let language_button = widget::button::link(
            CosmicScreenMessages::ToggleLanguageHint {
                current_language: self.current_language,
            }
            .to_fluent_string(),
        )
        .on_press(Message::ToggleLanguage);

        widget::column()
            .push(title)
            .push(language_button)
            .align_x(Alignment::Center)
            .apply(widget::container)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run_with_id(
                std::any::TypeId::of::<MySubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    _ = channel.send(Message::SubscriptionChannel).await;

                    futures_util::future::pending().await
                }),
            ),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::OpenRepositoryUrl => {
                _ = open::that_detached(REPOSITORY);
            },

            Message::SubscriptionChannel => {
                // For example purposes only.
            },

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            },

            Message::UpdateConfig(config) => {
                self.config = config;
            },

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {},
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                },
            },

            Message::ToggleLanguage => {
                let mut languages: Vec<example_shared_lib::Languages> =
                    example_shared_lib::Languages::iter().collect();
                languages.sort_by_key(|a| *a as isize);
                let current_index = languages
                    .iter()
                    .position(|&lang| lang == self.current_language)
                    .unwrap_or(0);
                let next_index = (current_index + 1) % languages.len();
                let next_language = languages[next_index];

                self.current_language = next_language;
                crate::i18n::change_locale(&next_language.into()).unwrap();
                self.rebuild_nav();
                return self.update_title();
            },
        }
        Task::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        // Infer and store the selected page, then activate it.
        if let Some(&page) = self.nav.data::<Page>(id) {
            self.current_page = page;
        }
        self.nav.activate(id);

        self.update_title()
    }
}

impl AppModel {
    /// The about page for this app.
    pub fn about(&self) -> Element<Message> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));

        let title = widget::text::title3(AppItems::Title.to_fluent_string());

        let hash = env!("VERGEN_GIT_SHA");
        let short_hash: String = hash.chars().take(7).collect();
        let date = env!("VERGEN_GIT_COMMIT_DATE");

        let link = widget::button::link(REPOSITORY)
            .on_press(Message::OpenRepositoryUrl)
            .padding(0);

        widget::column()
            .push(icon)
            .push(title)
            .push(link)
            .push(
                widget::button::link(
                    GitDescription {
                        hash: short_hash.as_str(),
                        date,
                    }
                    .to_fluent_string(),
                )
                .on_press(Message::LaunchUrl(format!("{REPOSITORY}/commits/{hash}")))
                .padding(0),
            )
            .align_x(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }

    /// Rebuilds the nav model with localized labels and preserves the active page.
    pub fn rebuild_nav(&mut self) {
        let active_page = self.current_page;

        let mut nav = nav_bar::Model::default();

        match active_page {
            Page::Page1 => {
                nav.insert()
                    .text(PageNumber::new(1).to_fluent_string())
                    .data::<Page>(Page::Page1)
                    .icon(icon::from_name("applications-science-symbolic"))
                    .activate();

                nav.insert()
                    .text(PageNumber::new(2).to_fluent_string())
                    .data::<Page>(Page::Page2)
                    .icon(icon::from_name("applications-system-symbolic"));

                nav.insert()
                    .text(PageNumber::new(3).to_fluent_string())
                    .data::<Page>(Page::Page3)
                    .icon(icon::from_name("applications-games-symbolic"));
            },
            Page::Page2 => {
                nav.insert()
                    .text(PageNumber::new(1).to_fluent_string())
                    .data::<Page>(Page::Page1)
                    .icon(icon::from_name("applications-science-symbolic"));

                nav.insert()
                    .text(PageNumber::new(2).to_fluent_string())
                    .data::<Page>(Page::Page2)
                    .icon(icon::from_name("applications-system-symbolic"))
                    .activate();

                nav.insert()
                    .text(PageNumber::new(3).to_fluent_string())
                    .data::<Page>(Page::Page3)
                    .icon(icon::from_name("applications-games-symbolic"));
            },
            Page::Page3 => {
                nav.insert()
                    .text(PageNumber::new(1).to_fluent_string())
                    .data::<Page>(Page::Page1)
                    .icon(icon::from_name("applications-science-symbolic"));

                nav.insert()
                    .text(PageNumber::new(2).to_fluent_string())
                    .data::<Page>(Page::Page2)
                    .icon(icon::from_name("applications-system-symbolic"));

                nav.insert()
                    .text(PageNumber::new(3).to_fluent_string())
                    .data::<Page>(Page::Page3)
                    .icon(icon::from_name("applications-games-symbolic"))
                    .activate();
            },
        }

        self.nav = nav;
    }

    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut window_title = AppItems::Title.to_fluent_string();

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}

#[derive(EsFluent)]
pub struct GitDescription<'a> {
    hash: &'a str,
    date: &'a str,
}

#[derive(EsFluent)]
pub enum AppItems {
    Title,
    Welcome,
    About,
    View,
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub enum CosmicScreenMessages {
    ToggleLanguageHint {
        current_language: example_shared_lib::Languages,
    },
}

#[derive(EsFluent)]
pub struct PageNumber {
    number: u32,
}

impl PageNumber {
    pub fn new(number: u32) -> Self {
        Self { number }
    }
}

/// The page to display in the application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Page {
    Page1,
    Page2,
    Page3,
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
        }
    }
}

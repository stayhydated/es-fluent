#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2024::*;
#[macro_use]
extern crate std;
pub mod first_example {
    use es_fluent::{EsFluent, EsFluentChoice};
    use strum::EnumIter;
    pub enum Hello<'a> {
        User { user_name: &'a str },
    }
    impl<'a> ::es_fluent::FluentDisplay for Hello<'a> {
        fn fluent_fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                Self::User { user_name } => {
                    let mut args = ::std::collections::HashMap::new();
                    args.insert(
                        "user_name",
                        ::es_fluent::FluentValue::from((*user_name)),
                    );
                    f.write_fmt(
                        format_args!(
                            "{0}",
                            ::es_fluent::localize("hello-User", Some(&args)),
                        ),
                    )
                }
            }
        }
    }
    impl<'a> From<&Hello<'a>> for ::es_fluent::FluentValue<'_> {
        fn from(value: &Hello<'a>) -> Self {
            use ::es_fluent::ToFluentString as _;
            value.to_fluent_string().into()
        }
    }
    impl<'a> From<Hello<'a>> for ::es_fluent::FluentValue<'_> {
        fn from(value: Hello<'a>) -> Self {
            (&value).into()
        }
    }
    #[fluent_choice(serialize_all = "snake_case")]
    pub enum Gender {
        Male,
        Female,
        Helicopter,
        Other,
    }
    ///An iterator over the variants of [Gender]
    #[allow(missing_copy_implementations)]
    pub struct GenderIter {
        idx: usize,
        back_idx: usize,
        marker: ::core::marker::PhantomData<()>,
    }
    impl ::core::fmt::Debug for GenderIter {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_struct("GenderIter").field("len", &self.len()).finish()
        }
    }
    impl GenderIter {
        fn get(&self, idx: usize) -> ::core::option::Option<Gender> {
            match idx {
                0usize => ::core::option::Option::Some(Gender::Male),
                1usize => ::core::option::Option::Some(Gender::Female),
                2usize => ::core::option::Option::Some(Gender::Helicopter),
                3usize => ::core::option::Option::Some(Gender::Other),
                _ => ::core::option::Option::None,
            }
        }
    }
    impl ::strum::IntoEnumIterator for Gender {
        type Iterator = GenderIter;
        fn iter() -> GenderIter {
            GenderIter {
                idx: 0,
                back_idx: 0,
                marker: ::core::marker::PhantomData,
            }
        }
    }
    impl Iterator for GenderIter {
        type Item = Gender;
        fn next(&mut self) -> ::core::option::Option<<Self as Iterator>::Item> {
            self.nth(0)
        }
        fn size_hint(&self) -> (usize, ::core::option::Option<usize>) {
            let t = if self.idx + self.back_idx >= 4usize {
                0
            } else {
                4usize - self.idx - self.back_idx
            };
            (t, Some(t))
        }
        fn nth(&mut self, n: usize) -> ::core::option::Option<<Self as Iterator>::Item> {
            let idx = self.idx + n + 1;
            if idx + self.back_idx > 4usize {
                self.idx = 4usize;
                ::core::option::Option::None
            } else {
                self.idx = idx;
                GenderIter::get(self, idx - 1)
            }
        }
    }
    impl ExactSizeIterator for GenderIter {
        fn len(&self) -> usize {
            self.size_hint().0
        }
    }
    impl DoubleEndedIterator for GenderIter {
        fn next_back(&mut self) -> ::core::option::Option<<Self as Iterator>::Item> {
            let back_idx = self.back_idx + 1;
            if self.idx + back_idx > 4usize {
                self.back_idx = 4usize;
                ::core::option::Option::None
            } else {
                self.back_idx = back_idx;
                GenderIter::get(self, 4usize - self.back_idx)
            }
        }
    }
    impl ::core::iter::FusedIterator for GenderIter {}
    impl Clone for GenderIter {
        fn clone(&self) -> GenderIter {
            GenderIter {
                idx: self.idx,
                back_idx: self.back_idx,
                marker: self.marker.clone(),
            }
        }
    }
    impl ::es_fluent::FluentDisplay for Gender {
        fn fluent_fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                Self::Male => {
                    f.write_fmt(
                        format_args!("{0}", ::es_fluent::localize("gender-Male", None)),
                    )
                }
                Self::Female => {
                    f.write_fmt(
                        format_args!("{0}", ::es_fluent::localize("gender-Female", None)),
                    )
                }
                Self::Helicopter => {
                    f.write_fmt(
                        format_args!(
                            "{0}",
                            ::es_fluent::localize("gender-Helicopter", None),
                        ),
                    )
                }
                Self::Other => {
                    f.write_fmt(
                        format_args!("{0}", ::es_fluent::localize("gender-Other", None)),
                    )
                }
            }
        }
    }
    impl From<&Gender> for ::es_fluent::FluentValue<'_> {
        fn from(value: &Gender) -> Self {
            use ::es_fluent::ToFluentString as _;
            value.to_fluent_string().into()
        }
    }
    impl From<Gender> for ::es_fluent::FluentValue<'_> {
        fn from(value: Gender) -> Self {
            (&value).into()
        }
    }
    impl ::es_fluent::EsFluentChoice for Gender {
        fn as_fluent_choice(&self) -> &'static str {
            match self {
                Self::Male => "male",
                Self::Female => "female",
                Self::Helicopter => "helicopter",
                Self::Other => "other",
            }
        }
    }
    pub enum Shared<'a> {
        Photos {
            user_name: &'a str,
            /// of course! we get this data
            /// from a reference to a reference to a reference to a reference to a u32
            photo_count: &'a &'a &'a &'a u32,
            #[fluent(choice)]
            user_gender: &'a Gender,
        },
    }
    impl<'a> ::es_fluent::FluentDisplay for Shared<'a> {
        fn fluent_fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                Self::Photos { user_name, photo_count, user_gender } => {
                    let mut args = ::std::collections::HashMap::new();
                    args.insert(
                        "user_name",
                        ::es_fluent::FluentValue::from((*user_name)),
                    );
                    args.insert(
                        "photo_count",
                        ::es_fluent::FluentValue::from((*(*(*(*photo_count))))),
                    );
                    args.insert(
                        "user_gender",
                        ::es_fluent::FluentValue::from(user_gender.as_fluent_choice()),
                    );
                    f.write_fmt(
                        format_args!(
                            "{0}",
                            ::es_fluent::localize("shared-Photos", Some(&args)),
                        ),
                    )
                }
            }
        }
    }
    impl<'a> From<&Shared<'a>> for ::es_fluent::FluentValue<'_> {
        fn from(value: &Shared<'a>) -> Self {
            use ::es_fluent::ToFluentString as _;
            value.to_fluent_string().into()
        }
    }
    impl<'a> From<Shared<'a>> for ::es_fluent::FluentValue<'_> {
        fn from(value: Shared<'a>) -> Self {
            (&value).into()
        }
    }
}
pub mod i18n {
    use es_fluent::{FluentManager, set_context, update_context};
    use es_fluent_macros::define_i18n_module;
    mod __es_fluent_generated {
        use es_fluent::{Localizer, LocalizationError};
        use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
        use fluent_bundle::concurrent::FluentBundle as ConcurrentFluentBundle;
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};
        use unic_langid::LanguageIdentifier;
        #[folder = "../i18n/"]
        struct Localizations;
        impl Localizations {
            fn matcher() -> rust_embed::utils::PathMatcher {
                const INCLUDES: &[&str] = &[];
                const EXCLUDES: &[&str] = &[];
                static PATH_MATCHER: ::std::sync::OnceLock<
                    rust_embed::utils::PathMatcher,
                > = ::std::sync::OnceLock::new();
                PATH_MATCHER
                    .get_or_init(|| rust_embed::utils::PathMatcher::new(
                        INCLUDES,
                        EXCLUDES,
                    ))
                    .clone()
            }
            /// Get an embedded file and its metadata.
            pub fn get(
                file_path: &str,
            ) -> ::std::option::Option<rust_embed::EmbeddedFile> {
                let rel_file_path = file_path.replace("\\", "/");
                let file_path = ::std::path::Path::new(
                        "/home/mark/Documents/GitHub/es-fluent/examples/first-example/../i18n/",
                    )
                    .join(&rel_file_path);
                let canonical_file_path = file_path.canonicalize().ok()?;
                if !canonical_file_path
                    .starts_with("/home/mark/Documents/GitHub/es-fluent/examples/i18n")
                {
                    let metadata = ::std::fs::symlink_metadata(&file_path).ok()?;
                    if !metadata.is_symlink() {
                        return ::std::option::Option::None;
                    }
                }
                let path_matcher = Self::matcher();
                if path_matcher.is_path_included(&rel_file_path) {
                    rust_embed::utils::read_file_from_fs(&canonical_file_path).ok()
                } else {
                    ::std::option::Option::None
                }
            }
            /// Iterates over the file paths in the folder.
            pub fn iter() -> impl ::std::iter::Iterator<
                Item = ::std::borrow::Cow<'static, str>,
            > {
                use ::std::path::Path;
                rust_embed::utils::get_files(
                        ::std::string::String::from(
                            "/home/mark/Documents/GitHub/es-fluent/examples/first-example/../i18n/",
                        ),
                        Self::matcher(),
                    )
                    .map(|e| ::std::borrow::Cow::from(e.rel_path))
            }
        }
        impl rust_embed::RustEmbed for Localizations {
            fn get(file_path: &str) -> ::std::option::Option<rust_embed::EmbeddedFile> {
                Localizations::get(file_path)
            }
            fn iter() -> rust_embed::Filenames {
                rust_embed::Filenames::Dynamic(
                    ::std::boxed::Box::new(Localizations::iter()),
                )
            }
        }
        pub struct FirstExampleLocalizer {
            bundle: Arc<Mutex<Option<ConcurrentFluentBundle<FluentResource>>>>,
        }
        impl FirstExampleLocalizer {
            pub fn new(fallback_language: LanguageIdentifier) -> Self {
                let bundle = Self::create_bundle(fallback_language.clone())
                    .expect(
                        "Failed to load this module's fallback language from embedded assets.",
                    );
                Self {
                    bundle: Arc::new(Mutex::new(Some(bundle))),
                }
            }
            fn create_bundle(
                lang: LanguageIdentifier,
            ) -> Option<ConcurrentFluentBundle<FluentResource>> {
                let mut bundle = ConcurrentFluentBundle::<
                    FluentResource,
                >::new_concurrent(
                    <[_]>::into_vec(::alloc::boxed::box_new([lang.clone()])),
                );
                let ftl_path = ::alloc::__export::must_use({
                    ::alloc::fmt::format(
                        format_args!("{0}/{1}.ftl", lang, "first-example"),
                    )
                });
                let file = Localizations::get(&ftl_path)
                    .unwrap_or_else(|| {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "FTL file \'{0}\' not found in embedded assets",
                                ftl_path,
                            ),
                        );
                    });
                let content = std::str::from_utf8(file.data.as_ref()).ok()?;
                let res = FluentResource::try_new(content.to_string())
                    .expect("Failed to parse FTL file.");
                bundle.add_resource(res).ok()?;
                Some(bundle)
            }
        }
        impl Localizer for FirstExampleLocalizer {
            fn select_language(
                &self,
                lang: &LanguageIdentifier,
            ) -> Result<(), LocalizationError> {
                if let Some(bundle) = Self::create_bundle(lang.clone()) {
                    let mut guard = self.bundle.lock().unwrap();
                    *guard = Some(bundle);
                    Ok(())
                } else {
                    Err(LocalizationError::LanguageNotSupported(lang.clone()))
                }
            }
            fn localize<'a>(
                &self,
                id: &str,
                args: Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> Option<String> {
                let guard = self.bundle.lock().unwrap();
                let bundle = guard.as_ref()?;
                let msg = bundle.get_message(id)?;
                let pattern = msg.value()?;
                let mut errors = Vec::new();
                let fluent_args = args
                    .map(|args| {
                        let mut fa = FluentArgs::new();
                        for (key, value) in args {
                            fa.set(*key, value.clone());
                        }
                        fa
                    });
                let value = bundle
                    .format_pattern(pattern, fluent_args.as_ref(), &mut errors);
                if !errors.is_empty() {
                    {
                        {
                            let lvl = ::log::Level::Error;
                            if lvl <= ::log::STATIC_MAX_LEVEL
                                && lvl <= ::log::max_level()
                            {
                                ::log::__private_api::log(
                                    { ::log::__private_api::GlobalLogger },
                                    format_args!(
                                        "Fluent formatting errors for message \'{0}\': {1:?}",
                                        id,
                                        errors,
                                    ),
                                    lvl,
                                    &(
                                        "first_example::i18n::__es_fluent_generated",
                                        "first_example::i18n::__es_fluent_generated",
                                        ::log::__private_api::loc(),
                                    ),
                                    (),
                                );
                            }
                        }
                    };
                }
                Some(value.to_string())
            }
        }
    }
    struct FirstExampleI18nModule;
    impl es_fluent::I18nModule for FirstExampleI18nModule {
        fn name(&self) -> &'static str {
            "first-example"
        }
        fn create_localizer(&self) -> Box<dyn es_fluent::Localizer> {
            let fallback_lang = {
                #[allow(dead_code)]
                enum ProcMacroHack {
                    Value = ("\"en\"", 0).1,
                }
                unsafe {
                    ::unic_langid_macros::LanguageIdentifier::from_raw_parts_unchecked(
                        unsafe {
                            ::unic_langid_macros::subtags::Language::from_raw_unchecked(
                                28261u64,
                            )
                        },
                        None,
                        None,
                        None,
                    )
                }
            };
            Box::new(
                self::__es_fluent_generated::FirstExampleLocalizer::new(fallback_lang),
            )
        }
    }
    #[allow(non_upper_case_globals)]
    const _: () = {
        static __INVENTORY: ::inventory::Node = ::inventory::Node {
            value: &{ &FirstExampleI18nModule as &dyn es_fluent::I18nModule },
            next: ::inventory::core::cell::UnsafeCell::new(
                ::inventory::core::option::Option::None,
            ),
        };
        #[link_section = ".text.startup"]
        unsafe extern "C" fn __ctor() {
            unsafe { ::inventory::ErasedNode::submit(__INVENTORY.value, &__INVENTORY) }
        }
        #[used]
        #[link_section = ".init_array"]
        static __CTOR: unsafe extern "C" fn() = __ctor;
    };
    pub fn init() -> FluentManager {
        let manager = FluentManager::new_with_discovered_modules();
        set_context(manager.clone());
        manager
    }
    pub fn change_locale(
        manager: &mut FluentManager,
        language: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let lang_id: unic_langid::LanguageIdentifier = language.parse()?;
        manager.select_language(&lang_id);
        update_context(|ctx| *ctx = manager.clone());
        Ok(())
    }
}

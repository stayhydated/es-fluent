use super::super::FallbackValidationDerive;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TestDisabledCfg {
    True,
    False,
    Unknown,
}

pub(crate) fn attributes_require_test(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .filter_map(|attribute| attribute.parse_args::<syn::Meta>().ok())
        .any(|predicate| cfg_with_test_disabled(&predicate) == TestDisabledCfg::False)
}

pub(super) fn attributes_create_test_context(attributes: &[syn::Attribute]) -> bool {
    attributes_require_test(attributes)
        || attributes
            .iter()
            .any(|attribute| attribute.path().is_ident("test"))
}

pub(crate) fn attributes_enable_test_only_derive(
    attributes: &[syn::Attribute],
    derive: Option<FallbackValidationDerive>,
) -> bool {
    let Some(derive) = derive else {
        return false;
    };
    attributes.iter().any(|attribute| {
        let syn::Meta::List(list) = &attribute.meta else {
            return false;
        };
        if !list.path.is_ident("cfg_attr") {
            return false;
        }
        let Some(arguments) = cfg_predicates(list) else {
            return false;
        };
        let Some((predicate, applied_attributes)) = arguments.split_first() else {
            return false;
        };
        cfg_with_test_disabled(predicate) == TestDisabledCfg::False
            && applied_attributes
                .iter()
                .any(|attribute| meta_derives(attribute, derive))
    })
}

fn meta_derives(meta: &syn::Meta, derive: FallbackValidationDerive) -> bool {
    let syn::Meta::List(list) = meta else {
        return false;
    };
    if !list.path.is_ident("derive") {
        return false;
    }

    use syn::parse::Parser as _;

    syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()
        .is_some_and(|paths| {
            paths.iter().any(|path| {
                path.segments
                    .last()
                    .is_some_and(|segment| segment.ident == derive.name())
            })
        })
}

fn cfg_with_test_disabled(predicate: &syn::Meta) -> TestDisabledCfg {
    match predicate {
        syn::Meta::Path(path) if path.is_ident("test") => TestDisabledCfg::False,
        syn::Meta::Path(_) | syn::Meta::NameValue(_) => TestDisabledCfg::Unknown,
        syn::Meta::List(list) if list.path.is_ident("all") => {
            let Some(predicates) = cfg_predicates(list) else {
                return TestDisabledCfg::Unknown;
            };
            if predicates
                .iter()
                .any(|predicate| cfg_with_test_disabled(predicate) == TestDisabledCfg::False)
            {
                TestDisabledCfg::False
            } else if predicates
                .iter()
                .all(|predicate| cfg_with_test_disabled(predicate) == TestDisabledCfg::True)
            {
                TestDisabledCfg::True
            } else {
                TestDisabledCfg::Unknown
            }
        },
        syn::Meta::List(list) if list.path.is_ident("any") => {
            let Some(predicates) = cfg_predicates(list) else {
                return TestDisabledCfg::Unknown;
            };
            if predicates
                .iter()
                .any(|predicate| cfg_with_test_disabled(predicate) == TestDisabledCfg::True)
            {
                TestDisabledCfg::True
            } else if predicates
                .iter()
                .all(|predicate| cfg_with_test_disabled(predicate) == TestDisabledCfg::False)
            {
                TestDisabledCfg::False
            } else {
                TestDisabledCfg::Unknown
            }
        },
        syn::Meta::List(list) if list.path.is_ident("not") => {
            let Some(predicates) = cfg_predicates(list) else {
                return TestDisabledCfg::Unknown;
            };
            let [predicate] = predicates.as_slice() else {
                return TestDisabledCfg::Unknown;
            };
            match cfg_with_test_disabled(predicate) {
                TestDisabledCfg::True => TestDisabledCfg::False,
                TestDisabledCfg::False => TestDisabledCfg::True,
                TestDisabledCfg::Unknown => TestDisabledCfg::Unknown,
            }
        },
        syn::Meta::List(_) => TestDisabledCfg::Unknown,
    }
}

fn cfg_predicates(list: &syn::MetaList) -> Option<Vec<syn::Meta>> {
    use syn::parse::Parser as _;

    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()
        .map(IntoIterator::into_iter)
        .map(Iterator::collect)
}

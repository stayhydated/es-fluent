use proc_macro2::Span;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use syn::spanned::Spanned as _;
use syn::visit::Visit as _;

const MANAGER_CRATE_ROOTS: &[&str] = &[
    "es_fluent_manager_embedded",
    "es_fluent_manager_dioxus",
    "es_fluent_manager_bevy",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceTarget<'a> {
    Call(&'static str),
    CallWithRoots(&'static str, &'a [String]),
    Macro(&'static str, Option<&'a [String]>),
}

impl<'a> SourceTarget<'a> {
    pub(crate) fn build_helper_call(roots: &'a [String]) -> Self {
        if roots == ["es_fluent_build"] {
            Self::Call("track_i18n_assets")
        } else {
            Self::CallWithRoots("track_i18n_assets", roots)
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Call(name) | Self::CallWithRoots(name, _) | Self::Macro(name, _) => name,
        }
    }

    fn call_name(self) -> Option<&'static str> {
        match self {
            Self::Call(name) | Self::CallWithRoots(name, _) => Some(name),
            Self::Macro(_, _) => None,
        }
    }

    fn is_call(self) -> bool {
        self.call_name().is_some()
    }

    fn is_expected_path(self, path: &syn::Path) -> bool {
        let mut segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string());
        let Some(root) = segments.next() else {
            return false;
        };
        let Some(name) = segments.next() else {
            return false;
        };
        if segments.next().is_some() || name != self.name() {
            return false;
        }

        self.is_expected_root(&root)
    }

    fn is_expected_root(self, root: &str) -> bool {
        match self {
            Self::Call(_) => root == "es_fluent_build",
            Self::CallWithRoots(_, roots) => roots.iter().any(|expected| expected == root),
            Self::Macro(_, roots) => roots.map_or_else(
                || MANAGER_CRATE_ROOTS.contains(&root),
                |roots| roots.iter().any(|expected| expected == root),
            ),
        }
    }

    fn is_unexpected_manager_root(self, root: &str) -> bool {
        matches!(self, Self::Macro(_, Some(_)))
            && MANAGER_CRATE_ROOTS.contains(&root)
            && !self.is_expected_root(root)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceEvidence {
    pub(crate) path: PathBuf,
    pub(crate) line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InspectionOutcome {
    Found(SourceEvidence),
    NotFound,
    Indeterminate(String),
}

#[derive(Debug, Default)]
pub(crate) struct SourceGraph {
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) lexical_paths: Vec<PathBuf>,
    pub(crate) watch_dirs: Vec<PathBuf>,
    pub(crate) indeterminate_reasons: Vec<String>,
    evidence: Option<MatchedEvidence>,
    sources: Vec<ParsedSource>,
    evidences: Vec<MatchedEvidence>,
}

#[derive(Debug)]
struct MatchedEvidence {
    location: SourceEvidence,
    verified: bool,
    conditional: bool,
    function: Option<FunctionLocation>,
    execution_uncertain: bool,
    reachable: bool,
    unexpected_manager_root: bool,
}

fn evidence_rank(evidence: &MatchedEvidence) -> (bool, bool, bool, bool) {
    (
        evidence.unexpected_manager_root,
        evidence.reachable,
        evidence.verified && !evidence.conditional,
        evidence.verified,
    )
}

struct PendingSource {
    path: PathBuf,
    module_dir: PathBuf,
    module_path: Vec<String>,
    conditional: bool,
    allowed_root: PathBuf,
    explicit_path: bool,
}

struct ModuleCollector<'a> {
    allowed_root: &'a Path,
    pending: &'a mut Vec<PendingSource>,
    reasons: &'a mut Vec<String>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct FunctionLocation {
    path: PathBuf,
    line: usize,
    column: usize,
}

#[derive(Debug)]
struct ParsedSource {
    path: PathBuf,
    module_path: Vec<String>,
    file: syn::File,
}

#[derive(Debug, Default)]
struct ReachabilityAnalysis {
    reachable_functions: HashSet<FunctionLocation>,
    execution_uncertain_functions: HashSet<FunctionLocation>,
    indeterminate_reasons: Vec<String>,
}

#[derive(Debug)]
struct FunctionDefinition {
    location: FunctionLocation,
    name: String,
    module_path: Vec<String>,
    calls: Vec<FunctionCall>,
}

#[derive(Debug)]
struct FunctionCall {
    path: Vec<String>,
    execution_uncertain: bool,
}

pub(crate) fn inspect(
    entry_path: &Path,
    package_root: &Path,
    target: SourceTarget<'_>,
) -> InspectionOutcome {
    let graph = inspect_source_graph(entry_path, package_root, Some(target));
    if let Some(evidence) = graph.evidence {
        if evidence.execution_uncertain {
            return InspectionOutcome::Indeterminate(format!(
                "the matching invocation at {}:{} is under control flow that could not be proven to execute",
                evidence.location.path.display(),
                evidence.location.line
            ));
        }
        if !evidence.reachable {
            return InspectionOutcome::Indeterminate(format!(
                "the matching invocation at {}:{} could not be proven reachable from `main`",
                evidence.location.path.display(),
                evidence.location.line
            ));
        }
        if evidence.conditional {
            return InspectionOutcome::Indeterminate(format!(
                "the matching invocation at {}:{} is conditionally compiled",
                evidence.location.path.display(),
                evidence.location.line
            ));
        }
        if !evidence.verified {
            if evidence.unexpected_manager_root {
                return InspectionOutcome::NotFound;
            }
            return InspectionOutcome::Indeterminate(format!(
                "the `{}` invocation could not be resolved to the expected es-fluent dependency",
                target.name()
            ));
        }
        if !graph.indeterminate_reasons.is_empty() {
            return InspectionOutcome::Indeterminate(graph.indeterminate_reasons.join("; "));
        }
        return InspectionOutcome::Found(evidence.location);
    }
    if graph.indeterminate_reasons.is_empty() {
        InspectionOutcome::NotFound
    } else {
        InspectionOutcome::Indeterminate(graph.indeterminate_reasons.join("; "))
    }
}

pub(crate) fn reachable_source_graph(entry_path: &Path, package_root: &Path) -> SourceGraph {
    inspect_source_graph(entry_path, package_root, None)
}

mod evidence;
mod graph;
mod reachability;

use evidence::{
    EvidenceVisitor, diverging_function_names, statement_may_skip_following,
    statement_unconditionally_terminates,
};
use graph::inspect_source_graph;
use reachability::{analyze_reachability, has_conditional_attr};

#[cfg(test)]
mod tests;

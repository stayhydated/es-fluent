use proc_macro2::Span;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use syn::spanned::Spanned as _;
use syn::visit::Visit as _;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceTarget {
    Call(&'static str),
    Macro(&'static str),
}

impl SourceTarget {
    fn name(self) -> &'static str {
        match self {
            Self::Call(name) | Self::Macro(name) => name,
        }
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

        match self {
            Self::Call(_) => root == "es_fluent_build",
            Self::Macro(_) => matches!(
                root.as_str(),
                "es_fluent_manager_embedded"
                    | "es_fluent_manager_dioxus"
                    | "es_fluent_manager_bevy"
            ),
        }
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
}

fn evidence_rank(evidence: &MatchedEvidence) -> (bool, bool, bool) {
    (
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
    indeterminate_reasons: Vec<String>,
}

#[derive(Debug)]
struct FunctionDefinition {
    location: FunctionLocation,
    name: String,
    module_path: Vec<String>,
    calls: Vec<Vec<String>>,
}

pub(crate) fn inspect(
    entry_path: &Path,
    allowed_root: &Path,
    target: SourceTarget,
) -> InspectionOutcome {
    let graph = inspect_source_graph(entry_path, allowed_root, Some(target));
    if let Some(evidence) = graph.evidence {
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
            return InspectionOutcome::Indeterminate(format!(
                "the `{}` invocation could not be resolved to the expected es-fluent dependency",
                target.name()
            ));
        }
        return InspectionOutcome::Found(evidence.location);
    }
    if graph.indeterminate_reasons.is_empty() {
        InspectionOutcome::NotFound
    } else {
        InspectionOutcome::Indeterminate(graph.indeterminate_reasons.join("; "))
    }
}

pub(crate) fn reachable_source_graph(entry_path: &Path, allowed_root: &Path) -> SourceGraph {
    inspect_source_graph(entry_path, allowed_root, None)
}

fn inspect_source_graph(
    entry_path: &Path,
    allowed_root: &Path,
    target: Option<SourceTarget>,
) -> SourceGraph {
    let root = std::fs::canonicalize(allowed_root).unwrap_or_else(|_| allowed_root.to_path_buf());
    let module_dir = entry_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let mut pending = vec![PendingSource {
        path: entry_path.to_path_buf(),
        module_dir,
        module_path: Vec::new(),
        conditional: false,
    }];
    let mut visited = HashSet::new();
    let mut graph = SourceGraph::default();

    while let Some(source) = pending.pop() {
        let canonical = match std::fs::canonicalize(&source.path) {
            Ok(path) => path,
            Err(error) => {
                graph.indeterminate_reasons.push(format!(
                    "failed to resolve {}: {error}",
                    source.path.display()
                ));
                continue;
            },
        };
        if !canonical.starts_with(&root) {
            graph.indeterminate_reasons.push(format!(
                "{} resolves outside {}",
                source.path.display(),
                allowed_root.display()
            ));
            continue;
        }
        if !visited.insert(canonical.clone()) {
            continue;
        }
        graph.paths.push(canonical.clone());

        let source_text = match std::fs::read_to_string(&canonical) {
            Ok(source_text) => source_text,
            Err(error) => {
                graph
                    .indeterminate_reasons
                    .push(format!("failed to read {}: {error}", canonical.display()));
                continue;
            },
        };
        let file = match syn::parse_file(&source_text) {
            Ok(file) => file,
            Err(error) => {
                graph
                    .indeterminate_reasons
                    .push(format!("failed to parse {}: {error}", canonical.display()));
                continue;
            },
        };

        graph.sources.push(ParsedSource {
            path: canonical.clone(),
            module_path: source.module_path.clone(),
            file,
        });
        let file = &graph.sources.last().expect("just-pushed source").file;

        let mut visitor = EvidenceVisitor::new(target, &canonical, source.conditional);
        visitor.visit_file(file);
        graph.evidences.extend(visitor.evidences);
        graph
            .indeterminate_reasons
            .extend(visitor.indeterminate_reasons);

        collect_pending_modules(
            &file.items,
            &canonical,
            &source.module_dir,
            &source.module_path,
            source.conditional,
            &mut pending,
            &mut graph.indeterminate_reasons,
        );
        for include in visitor.includes {
            let Some(parent) = canonical.parent() else {
                graph.indeterminate_reasons.push(format!(
                    "could not resolve include from {}",
                    canonical.display()
                ));
                continue;
            };
            match include.path {
                Some(path) => pending.push(PendingSource {
                    path: parent.join(path),
                    module_dir: source.module_dir.clone(),
                    module_path: source.module_path.clone(),
                    conditional: include.conditional,
                }),
                None => graph.indeterminate_reasons.push(format!(
                    "non-literal include! at {}:{}",
                    canonical.display(),
                    include.line
                )),
            }
        }
    }

    if matches!(target, Some(SourceTarget::Call(_))) {
        let analysis = analyze_reachability(&graph.sources, entry_path);
        graph
            .indeterminate_reasons
            .extend(analysis.indeterminate_reasons);
        for evidence in &mut graph.evidences {
            evidence.reachable = evidence
                .function
                .as_ref()
                .is_some_and(|function| analysis.reachable_functions.contains(function))
                && !evidence.execution_uncertain;
        }
    } else {
        for evidence in &mut graph.evidences {
            evidence.reachable = true;
        }
    }
    graph.evidence = std::mem::take(&mut graph.evidences)
        .into_iter()
        .max_by(|left, right| evidence_rank(left).cmp(&evidence_rank(right)));

    graph.paths.sort();
    graph.paths.dedup();
    graph.indeterminate_reasons.sort();
    graph.indeterminate_reasons.dedup();
    graph
}

fn collect_pending_modules(
    items: &[syn::Item],
    current_file: &Path,
    module_dir: &Path,
    module_path: &[String],
    inherited_conditional: bool,
    pending: &mut Vec<PendingSource>,
    reasons: &mut Vec<String>,
) {
    for item in items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        let conditional = inherited_conditional || has_conditional_attr(&module.attrs);
        let child_module_dir = module_dir.join(module.ident.to_string());
        let mut child_module_path = module_path.to_vec();
        child_module_path.push(module.ident.to_string());
        if let Some((_, items)) = &module.content {
            collect_pending_modules(
                items,
                current_file,
                &child_module_dir,
                &child_module_path,
                conditional,
                pending,
                reasons,
            );
            continue;
        }

        let explicit_path = module
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("path"));
        let resolved = if let Some(attribute) = explicit_path {
            match &attribute.meta {
                syn::Meta::NameValue(value) => match &value.value {
                    syn::Expr::Lit(expression) => match &expression.lit {
                        syn::Lit::Str(path) => Some(module_dir.join(path.value())),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            }
        } else {
            let flat = module_dir.join(format!("{}.rs", module.ident));
            let nested = child_module_dir.join("mod.rs");
            match (flat.is_file(), nested.is_file()) {
                (true, false) => Some(flat),
                (false, true) => Some(nested),
                (true, true) => {
                    reasons.push(format!(
                        "module `{}` from {} has both conventional source paths",
                        module.ident,
                        current_file.display()
                    ));
                    None
                },
                (false, false) => None,
            }
        };

        let Some(path) = resolved else {
            reasons.push(format!(
                "could not resolve module `{}` declared in {}:{}",
                module.ident,
                current_file.display(),
                module.ident.span().start().line
            ));
            continue;
        };
        let next_module_dir = if path.file_name().is_some_and(|name| name == "mod.rs") {
            path.parent()
                .map(Path::to_path_buf)
                .unwrap_or(child_module_dir)
        } else if explicit_path.is_some() {
            let stem = path
                .file_stem()
                .map(std::ffi::OsStr::to_os_string)
                .unwrap_or_else(|| module.ident.to_string().into());
            path.parent()
                .map(|parent| parent.join(stem))
                .unwrap_or(child_module_dir)
        } else {
            child_module_dir
        };
        pending.push(PendingSource {
            path,
            module_dir: next_module_dir,
            module_path: child_module_path,
            conditional,
        });
    }
}

fn analyze_reachability(sources: &[ParsedSource], entry_path: &Path) -> ReachabilityAnalysis {
    let mut definitions = Vec::new();
    for source in sources {
        collect_function_definitions(
            &source.file.items,
            &source.path,
            &source.module_path,
            &mut definitions,
        );
    }

    let entry_path = std::fs::canonicalize(entry_path).unwrap_or_else(|_| entry_path.to_path_buf());
    let entry_points = definitions
        .iter()
        .enumerate()
        .filter(|(_, definition)| definition.name == "main" && definition.module_path.is_empty())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if entry_points.is_empty() {
        return ReachabilityAnalysis {
            reachable_functions: HashSet::new(),
            indeterminate_reasons: vec![format!(
                "could not identify `fn main` in custom-build target {}",
                entry_path.display()
            )],
        };
    }

    let by_name = definitions.iter().enumerate().fold(
        HashMap::<String, Vec<usize>>::new(),
        |mut by_name, (index, definition)| {
            by_name
                .entry(definition.name.clone())
                .or_default()
                .push(index);
            by_name
        },
    );
    let mut reachable_functions = HashSet::new();
    let mut pending = VecDeque::from(entry_points);
    let mut indeterminate_reasons = Vec::new();
    while let Some(index) = pending.pop_front() {
        let definition = &definitions[index];
        if !reachable_functions.insert(definition.location.clone()) {
            continue;
        }
        for call in &definition.calls {
            if call.is_empty() {
                continue;
            }
            let candidates = resolve_local_functions(&definitions, &by_name, definition, call);
            if candidates.len() == 1 {
                pending.push_back(candidates[0]);
            } else if candidates.len() > 1 {
                indeterminate_reasons.push(format!(
                    "could not resolve local function call `{}` from {}:{}",
                    call.join("::"),
                    definition.location.path.display(),
                    definition.location.line
                ));
            }
        }
    }

    ReachabilityAnalysis {
        reachable_functions,
        indeterminate_reasons,
    }
}

fn collect_function_definitions(
    items: &[syn::Item],
    path: &Path,
    module_path: &[String],
    definitions: &mut Vec<FunctionDefinition>,
) {
    for item in items {
        match item {
            syn::Item::Fn(function) => {
                add_function_definition(function, path, module_path, definitions);
            },
            syn::Item::Mod(module) => {
                let Some((_, items)) = &module.content else {
                    continue;
                };
                let mut nested_module_path = module_path.to_vec();
                nested_module_path.push(module.ident.to_string());
                collect_function_definitions(items, path, &nested_module_path, definitions);
            },
            _ => {},
        }
    }
}

fn add_function_definition(
    function: &syn::ItemFn,
    path: &Path,
    module_path: &[String],
    definitions: &mut Vec<FunctionDefinition>,
) {
    let mut calls = Vec::new();
    let mut visitor = FunctionCallVisitor { calls: &mut calls };
    visitor.visit_block(&function.block);
    definitions.push(FunctionDefinition {
        location: FunctionLocation {
            path: path.to_path_buf(),
            line: function.sig.ident.span().start().line,
            column: function.sig.ident.span().start().column,
        },
        name: function.sig.ident.to_string(),
        module_path: module_path.to_vec(),
        calls,
    });
}

struct FunctionCallVisitor<'a> {
    calls: &'a mut Vec<Vec<String>>,
}

impl<'ast> syn::visit::Visit<'ast> for FunctionCallVisitor<'_> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = &*call.func {
            self.calls.push(
                function
                    .path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect(),
            );
        }
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_item_fn(&mut self, _function: &'ast syn::ItemFn) {}

    fn visit_expr_closure(&mut self, _closure: &'ast syn::ExprClosure) {}
}

fn resolve_local_functions(
    definitions: &[FunctionDefinition],
    by_name: &HashMap<String, Vec<usize>>,
    caller: &FunctionDefinition,
    call: &[String],
) -> Vec<usize> {
    let Some(name) = call.last() else {
        return Vec::new();
    };
    let Some(candidates) = by_name.get(name) else {
        return Vec::new();
    };

    let mut module_path = caller.module_path.clone();
    let mut relative = call;
    if call.first().is_some_and(|segment| segment == "crate") {
        module_path.clear();
        relative = &call[1..];
    } else if call.first().is_some_and(|segment| segment == "self") {
        relative = &call[1..];
    } else if call.first().is_some_and(|segment| segment == "super") {
        module_path.pop();
        relative = &call[1..];
    }
    if relative.len() > 1 {
        module_path.extend(relative[..relative.len() - 1].iter().cloned());
    }

    let qualified = candidates
        .iter()
        .copied()
        .filter(|index| definitions[*index].module_path == module_path)
        .collect::<Vec<_>>();
    if !qualified.is_empty() {
        return qualified;
    }
    Vec::new()
}

fn has_conditional_attr(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
}

struct IncludeSource {
    path: Option<PathBuf>,
    line: usize,
    conditional: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct ImportResolution {
    verified: bool,
    uncertain: bool,
}

impl ImportResolution {
    fn merge(&mut self, other: Self) {
        self.verified |= other.verified;
        self.uncertain |= other.uncertain;
    }
}

struct EvidenceVisitor<'a> {
    target: Option<SourceTarget>,
    current_file: &'a Path,
    conditional_depth: usize,
    current_function: Option<FunctionLocation>,
    execution_uncertain_depth: usize,
    evidences: Vec<MatchedEvidence>,
    includes: Vec<IncludeSource>,
    scopes: Vec<ImportResolution>,
    indeterminate_reasons: Vec<String>,
}

impl<'a> EvidenceVisitor<'a> {
    fn new(target: Option<SourceTarget>, current_file: &'a Path, conditional: bool) -> Self {
        Self {
            target,
            current_file,
            conditional_depth: usize::from(conditional),
            current_function: None,
            execution_uncertain_depth: 0,
            evidences: Vec::new(),
            includes: Vec::new(),
            scopes: Vec::new(),
            indeterminate_reasons: Vec::new(),
        }
    }

    fn record(&mut self, path: &syn::Path, span: Span, conditional: bool) {
        let verified = if path.segments.len() > 1 {
            self.target
                .is_some_and(|target| target.is_expected_path(path))
        } else {
            self.scopes.iter().rev().any(|scope| scope.verified)
        };
        let evidence = MatchedEvidence {
            location: SourceEvidence {
                path: self.current_file.to_path_buf(),
                line: span.start().line,
            },
            verified,
            conditional,
            function: self.current_function.clone(),
            execution_uncertain: self.execution_uncertain_depth > 0,
            reachable: false,
        };
        self.evidences.push(evidence);
    }

    fn visit_with_attributes(
        &mut self,
        attributes: &[syn::Attribute],
        visit: impl FnOnce(&mut Self),
    ) {
        let conditional = has_conditional_attr(attributes);
        self.conditional_depth += usize::from(conditional);
        visit(self);
        self.conditional_depth -= usize::from(conditional);
    }
}

impl<'ast> syn::visit::Visit<'ast> for EvidenceVisitor<'_> {
    fn visit_file(&mut self, file: &'ast syn::File) {
        let previous_scopes = std::mem::take(&mut self.scopes);
        self.scopes
            .push(imports_for_items(&file.items, self.target));
        for item in &file.items {
            self.visit_item(item);
        }
        self.scopes = previous_scopes;
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let (Some(SourceTarget::Call(target)), syn::Expr::Path(function)) =
            (self.target, &*call.func)
            && function
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == target)
        {
            self.record(
                &function.path,
                function.path.span(),
                self.conditional_depth > 0 || has_conditional_attr(&call.attrs),
            );
        }
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_macro(&mut self, invocation: &'ast syn::Macro) {
        if invocation.path.is_ident("include") {
            let path = syn::parse2::<syn::LitStr>(invocation.tokens.clone())
                .ok()
                .map(|literal| PathBuf::from(literal.value()));
            self.includes.push(IncludeSource {
                path,
                line: invocation.path.span().start().line,
                conditional: self.conditional_depth > 0,
            });
        }
        if let Some(SourceTarget::Macro(target)) = self.target
            && invocation
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == target)
        {
            self.record(
                &invocation.path,
                invocation.path.span(),
                self.conditional_depth > 0,
            );
        }
        syn::visit::visit_macro(self, invocation);
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        let previous_function = self.current_function.replace(FunctionLocation {
            path: self.current_file.to_path_buf(),
            line: function.sig.ident.span().start().line,
            column: function.sig.ident.span().start().column,
        });
        let previous_execution_uncertain = self.execution_uncertain_depth;
        self.execution_uncertain_depth = 0;
        self.visit_with_attributes(&function.attrs, |visitor| {
            syn::visit::visit_item_fn(visitor, function);
        });
        self.current_function = previous_function;
        self.execution_uncertain_depth = previous_execution_uncertain;
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        self.execution_uncertain_depth += 1;
        syn::visit::visit_expr_closure(self, closure);
        self.execution_uncertain_depth -= 1;
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        self.visit_with_attributes(&module.attrs, |visitor| {
            if let Some((_, items)) = &module.content {
                let previous_scopes = std::mem::take(&mut visitor.scopes);
                visitor
                    .scopes
                    .push(imports_for_items(items, visitor.target));
                for item in items {
                    visitor.visit_item(item);
                }
                visitor.scopes = previous_scopes;
            }
        });
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.scopes
            .push(imports_for_statements(&block.stmts, self.target));
        syn::visit::visit_block(self, block);
        self.scopes.pop();
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        self.visit_with_attributes(&item.attrs, |visitor| {
            if item.mac.path.is_ident("macro_rules")
                && visitor.target.is_some_and(|target| {
                    token_stream_contains_ident(&item.mac.tokens, target.name())
                })
            {
                visitor.indeterminate_reasons.push(format!(
                    "macro wrapper involving `{}` at {}:{}",
                    visitor.target.expect("checked target").name(),
                    visitor.current_file.display(),
                    item.mac.path.span().start().line
                ));
            }
            syn::visit::visit_item_macro(visitor, item);
        });
    }

    fn visit_expr_macro(&mut self, expression: &'ast syn::ExprMacro) {
        self.visit_with_attributes(&expression.attrs, |visitor| {
            syn::visit::visit_expr_macro(visitor, expression);
        });
    }

    fn visit_stmt_macro(&mut self, statement: &'ast syn::StmtMacro) {
        self.visit_with_attributes(&statement.attrs, |visitor| {
            syn::visit::visit_stmt_macro(visitor, statement);
        });
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        let resolution = import_resolution(item, self.target);
        if resolution.uncertain {
            self.indeterminate_reasons.push(format!(
                "unresolved import involving `{}` at {}:{}",
                self.target.expect("checked target").name(),
                self.current_file.display(),
                item.span().start().line
            ));
        }
        syn::visit::visit_item_use(self, item);
    }
}

fn imports_for_items(items: &[syn::Item], target: Option<SourceTarget>) -> ImportResolution {
    items
        .iter()
        .fold(ImportResolution::default(), |mut found, item| {
            if let syn::Item::Use(item) = item {
                found.merge(import_resolution(item, target));
            }
            found
        })
}

fn imports_for_statements(
    statements: &[syn::Stmt],
    target: Option<SourceTarget>,
) -> ImportResolution {
    statements
        .iter()
        .fold(ImportResolution::default(), |mut found, statement| {
            if let syn::Stmt::Item(syn::Item::Use(item)) = statement {
                found.merge(import_resolution(item, target));
            }
            found
        })
}

fn import_resolution(item: &syn::ItemUse, target: Option<SourceTarget>) -> ImportResolution {
    let Some(target) = target else {
        return ImportResolution::default();
    };
    let mut resolution = ImportResolution::default();
    inspect_use_tree(&item.tree, &mut Vec::new(), target, &mut resolution);
    if has_conditional_attr(&item.attrs) && resolution.verified {
        resolution.verified = false;
        resolution.uncertain = true;
    }
    resolution
}

fn inspect_use_tree(
    tree: &syn::UseTree,
    prefix: &mut Vec<syn::Ident>,
    target: SourceTarget,
    resolution: &mut ImportResolution,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.clone());
            inspect_use_tree(&path.tree, prefix, target, resolution);
            prefix.pop();
        },
        syn::UseTree::Name(name) => {
            prefix.push(name.ident.clone());
            if name.ident == target.name() {
                let path = syn::Path {
                    leading_colon: None,
                    segments: prefix.iter().cloned().map(syn::PathSegment::from).collect(),
                };
                if target.is_expected_path(&path) {
                    resolution.verified = true;
                } else {
                    resolution.uncertain = true;
                }
            }
            prefix.pop();
        },
        syn::UseTree::Rename(rename) => {
            if rename.ident == target.name() || rename.rename == target.name() {
                resolution.uncertain = true;
            }
        },
        syn::UseTree::Glob(_) => {},
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                inspect_use_tree(tree, prefix, target, resolution);
            }
        },
    }
}

fn token_stream_contains_ident(tokens: &proc_macro2::TokenStream, target: &str) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(ident) => ident == target,
        proc_macro2::TokenTree::Group(group) => {
            token_stream_contains_ident(&group.stream(), target)
        },
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn inspect_fixture(
        files: &[(&str, &str)],
        entry: &str,
        target: SourceTarget,
    ) -> InspectionOutcome {
        let temp = tempfile::tempdir().expect("tempdir");
        for (path, source) in files {
            let path = temp.path().join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create parent");
            }
            fs::write(path, source).expect("write source");
        }
        inspect(&temp.path().join(entry), temp.path(), target)
    }

    #[test]
    fn direct_qualified_and_imported_calls_are_found() {
        for source in [
            "fn main() { es_fluent_build::track_i18n_assets(); }",
            "use es_fluent_build::track_i18n_assets; fn main() { track_i18n_assets(); }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Found(_)
            ));
        }
    }

    #[test]
    fn qualified_and_imported_macros_are_found() {
        for source in [
            "es_fluent_manager_embedded::define_i18n_module!();",
            "use es_fluent_manager_embedded::define_i18n_module; define_i18n_module!();",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("lib.rs", source)],
                    "lib.rs",
                    SourceTarget::Macro("define_i18n_module")
                ),
                InspectionOutcome::Found(_)
            ));
        }
    }

    #[test]
    fn comments_and_strings_do_not_count_as_calls_or_macros() {
        assert_eq!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "fn main() { let _ = \"track_i18n_assets\"; } // track_i18n_assets()"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::NotFound
        );
        assert_eq!(
            inspect_fixture(
                &[(
                    "lib.rs",
                    "const _: &str = \"define_i18n_module!\"; /* define_i18n_module! */"
                )],
                "lib.rs",
                SourceTarget::Macro("define_i18n_module")
            ),
            InspectionOutcome::NotFound
        );
    }

    #[test]
    fn reachable_modules_and_literal_paths_are_followed() {
        let outcome = inspect_fixture(
            &[(
                "lib.rs",
                "mod inline { es_fluent_manager_embedded::define_i18n_module!(); }",
            )],
            "lib.rs",
            SourceTarget::Macro("define_i18n_module"),
        );
        assert!(matches!(outcome, InspectionOutcome::Found(_)));

        let outcome = inspect_fixture(
            &[
                ("lib.rs", "mod inline { mod registration; }"),
                (
                    "inline/registration.rs",
                    "es_fluent_manager_embedded::define_i18n_module!();",
                ),
            ],
            "lib.rs",
            SourceTarget::Macro("define_i18n_module"),
        );
        assert!(matches!(outcome, InspectionOutcome::Found(_)));

        let outcome = inspect_fixture(
            &[
                ("lib.rs", "mod registration;"),
                (
                    "registration/mod.rs",
                    "es_fluent_manager_embedded::define_i18n_module!();",
                ),
            ],
            "lib.rs",
            SourceTarget::Macro("define_i18n_module"),
        );
        assert!(matches!(outcome, InspectionOutcome::Found(_)));

        let outcome = inspect_fixture(
            &[
                (
                    "build.rs",
                    "#[path = \"support/assets.rs\"] mod assets; fn main() { assets::run(); }",
                ),
                (
                    "support/assets.rs",
                    "pub fn run() { es_fluent_build::track_i18n_assets(); }",
                ),
            ],
            "build.rs",
            SourceTarget::Call("track_i18n_assets"),
        );
        assert!(matches!(outcome, InspectionOutcome::Found(_)));
    }

    #[test]
    fn unreferenced_files_do_not_count() {
        assert_eq!(
            inspect_fixture(
                &[
                    ("lib.rs", "pub struct App;"),
                    ("unused.rs", "define_i18n_module!();")
                ],
                "lib.rs",
                SourceTarget::Macro("define_i18n_module")
            ),
            InspectionOutcome::NotFound
        );
    }

    #[test]
    fn build_helper_calls_must_be_reachable_from_main() {
        for source in [
            "fn unused() { es_fluent_build::track_i18n_assets(); } fn main() {}",
            "fn unused() { fn main() { es_fluent_build::track_i18n_assets(); } } fn main() {}",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Indeterminate(reason)
                    if reason.contains("could not be proven reachable")
            ));
        }

        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "fn configure() { es_fluent_build::track_i18n_assets(); } fn main() { configure(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Found(_)
        ));

        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "mod helper { pub fn configure() { es_fluent_build::track_i18n_assets(); } } fn configure() {} fn main() { configure(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("could not be proven reachable")
        ));
    }

    #[test]
    fn literal_includes_are_followed() {
        assert!(matches!(
            inspect_fixture(
                &[
                    (
                        "build.rs",
                        "include!(\"support.rs\"); fn main() { configure(); }"
                    ),
                    (
                        "support.rs",
                        "use es_fluent_build::track_i18n_assets; fn configure() { track_i18n_assets(); }"
                    )
                ],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Found(_)
        ));
    }

    #[test]
    fn dynamic_includes_aliases_and_conditional_matches_are_indeterminate() {
        for source in [
            "include!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));",
            "use es_fluent_build::track_i18n_assets as track; fn main() { track(); }",
            "fn track_i18n_assets() {} fn main() { track_i18n_assets(); }",
            "mod local { pub fn track_i18n_assets() {} } fn main() { local::track_i18n_assets(); }",
            "#[cfg(feature = \"i18n\")] use es_fluent_build::track_i18n_assets; fn main() { track_i18n_assets(); }",
            "#[cfg(feature = \"i18n\")] fn configure() { track_i18n_assets(); }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("lib.rs", source)],
                    "lib.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Indeterminate(_)
            ));
        }
        assert!(matches!(
            inspect_fixture(
                &[(
                    "lib.rs",
                    "use es_fluent_manager_embedded::define_i18n_module as define; define!();"
                )],
                "lib.rs",
                SourceTarget::Macro("define_i18n_module")
            ),
            InspectionOutcome::Indeterminate(_)
        ));
        assert!(matches!(
            inspect_fixture(
                &[(
                    "lib.rs",
                    "macro_rules! define_i18n_module { () => {} } define_i18n_module!();"
                )],
                "lib.rs",
                SourceTarget::Macro("define_i18n_module")
            ),
            InspectionOutcome::Indeterminate(_)
        ));
        assert!(matches!(
            inspect_fixture(
                &[(
                    "lib.rs",
                    "mod imported { use es_fluent_manager_embedded::define_i18n_module; } define_i18n_module!();"
                )],
                "lib.rs",
                SourceTarget::Macro("define_i18n_module")
            ),
            InspectionOutcome::Indeterminate(_)
        ));
        assert!(matches!(
            inspect_fixture(
                &[(
                    "lib.rs",
                    "fn setup() { #[cfg(feature = \"i18n\")] define_i18n_module!(); }"
                )],
                "lib.rs",
                SourceTarget::Macro("define_i18n_module")
            ),
            InspectionOutcome::Indeterminate(_)
        ));
        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "#[cfg(feature = \"other\")] fn conditional() { track_i18n_assets(); } fn main() { es_fluent_build::track_i18n_assets(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Found(_)
        ));
    }
}

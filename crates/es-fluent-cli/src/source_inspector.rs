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

fn inspect_source_graph(
    entry_path: &Path,
    package_root: &Path,
    target: Option<SourceTarget<'_>>,
) -> SourceGraph {
    let package_root =
        std::fs::canonicalize(package_root).unwrap_or_else(|_| package_root.to_path_buf());
    let canonical_entry =
        std::fs::canonicalize(entry_path).unwrap_or_else(|_| entry_path.to_path_buf());
    let source_root = package_root
        .ancestors()
        .find(|ancestor| canonical_entry.starts_with(ancestor))
        .map(Path::to_path_buf)
        .or_else(|| canonical_entry.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| package_root.clone());
    let module_dir = entry_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let mut pending = vec![PendingSource {
        path: entry_path.to_path_buf(),
        module_dir,
        module_path: Vec::new(),
        conditional: false,
        allowed_root: source_root,
        explicit_path: false,
    }];
    let mut visited = HashSet::new();
    let mut graph = SourceGraph::default();

    while let Some(source) = pending.pop() {
        graph.lexical_paths.push(source.path.clone());
        let canonical = match std::fs::canonicalize(&source.path) {
            Ok(path) => path,
            Err(error) => {
                if source.explicit_path
                    && let Some(directory) = nearest_existing_directory(&source.path)
                {
                    graph.watch_dirs.push(directory);
                }
                graph.indeterminate_reasons.push(format!(
                    "failed to resolve {}: {error}",
                    source.path.display()
                ));
                continue;
            },
        };
        if !canonical.starts_with(&source.allowed_root) && !source.explicit_path {
            if let Some(directory) = canonical.parent() {
                graph.watch_dirs.push(directory.to_path_buf());
            }
            graph.indeterminate_reasons.push(format!(
                "{} resolves outside {}",
                source.path.display(),
                source.allowed_root.display()
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

        let mut visitor = EvidenceVisitor::new(
            target,
            &canonical,
            source.conditional,
            diverging_function_names(file),
        );
        visitor.visit_file(file);
        graph.evidences.extend(visitor.evidences);
        graph
            .indeterminate_reasons
            .extend(visitor.indeterminate_reasons);

        let traversal_root = if source.explicit_path && !canonical.starts_with(&source.allowed_root)
        {
            canonical
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| source.allowed_root.clone())
        } else {
            source.allowed_root.clone()
        };
        {
            let mut collector = ModuleCollector {
                allowed_root: &traversal_root,
                pending: &mut pending,
                reasons: &mut graph.indeterminate_reasons,
            };
            collect_pending_modules(
                &file.items,
                &canonical,
                &source.module_dir,
                &source.module_path,
                source.conditional,
                &mut collector,
            );
        }
        for include in visitor.includes {
            let Some(parent) = source.path.parent() else {
                graph.indeterminate_reasons.push(format!(
                    "could not resolve include from {}",
                    canonical.display()
                ));
                continue;
            };
            match include.path {
                Some(path) => {
                    let path = parent.join(path);
                    let module_dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
                    pending.push(PendingSource {
                        path,
                        module_dir,
                        module_path: source.module_path.clone(),
                        conditional: include.conditional,
                        allowed_root: traversal_root.clone(),
                        explicit_path: true,
                    });
                },
                None => graph.indeterminate_reasons.push(format!(
                    "non-literal include! at {}:{}",
                    canonical.display(),
                    include.line
                )),
            }
        }
    }

    if target.is_some_and(SourceTarget::is_call) {
        let analysis = analyze_reachability(&graph.sources, entry_path);
        graph
            .indeterminate_reasons
            .extend(analysis.indeterminate_reasons);
        for evidence in &mut graph.evidences {
            evidence.execution_uncertain |= evidence
                .function
                .as_ref()
                .is_some_and(|function| analysis.execution_uncertain_functions.contains(function));
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
    graph.lexical_paths.sort();
    graph.lexical_paths.dedup();
    graph.watch_dirs.sort();
    graph.watch_dirs.dedup();
    graph.indeterminate_reasons.sort();
    graph.indeterminate_reasons.dedup();
    graph
}

fn nearest_existing_directory(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .filter(|ancestor| ancestor.parent().is_some())
        .find_map(|ancestor| {
            std::fs::canonicalize(ancestor)
                .ok()
                .filter(|path| path.is_dir())
        })
}

fn collect_pending_modules(
    items: &[syn::Item],
    current_file: &Path,
    module_dir: &Path,
    module_path: &[String],
    inherited_conditional: bool,
    collector: &mut ModuleCollector<'_>,
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
                collector,
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
                    collector.reasons.push(format!(
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
            collector.reasons.push(format!(
                "could not resolve module `{}` declared in {}:{}",
                module.ident,
                current_file.display(),
                module.ident.span().start().line
            ));
            continue;
        };
        let next_module_dir =
            if explicit_path.is_some() || path.file_name().is_some_and(|name| name == "mod.rs") {
                path.parent()
                    .map(Path::to_path_buf)
                    .unwrap_or(child_module_dir)
            } else {
                child_module_dir
            };
        collector.pending.push(PendingSource {
            path,
            module_dir: next_module_dir,
            module_path: child_module_path,
            conditional,
            allowed_root: collector.allowed_root.to_path_buf(),
            explicit_path: explicit_path.is_some(),
        });
    }
}

fn analyze_reachability(sources: &[ParsedSource], entry_path: &Path) -> ReachabilityAnalysis {
    let diverging_functions = sources
        .iter()
        .flat_map(|source| diverging_function_names(&source.file))
        .collect::<HashSet<_>>();
    let mut definitions = Vec::new();
    let mut module_imports = HashMap::<Vec<String>, FunctionImports>::new();
    for source in sources {
        collect_module_function_imports(
            &source.file.items,
            &source.module_path,
            &mut module_imports,
        );
    }
    for source in sources {
        collect_function_definitions(
            &source.file.items,
            &source.path,
            &source.module_path,
            &module_imports,
            &diverging_functions,
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
            execution_uncertain_functions: HashSet::new(),
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
    let mut definitely_reachable = HashSet::new();
    let mut uncertainly_reachable = HashSet::new();
    let mut pending = entry_points
        .into_iter()
        .map(|index| (index, false))
        .collect::<VecDeque<_>>();
    let mut indeterminate_reasons = Vec::new();
    while let Some((index, execution_uncertain)) = pending.pop_front() {
        let definition = &definitions[index];
        if execution_uncertain {
            if definitely_reachable.contains(&index) || !uncertainly_reachable.insert(index) {
                continue;
            }
        } else {
            if !definitely_reachable.insert(index) {
                continue;
            }
            uncertainly_reachable.remove(&index);
        }
        for call in &definition.calls {
            if call.path.is_empty() {
                continue;
            }
            let candidates =
                resolve_local_functions(&definitions, &by_name, definition, &call.path);
            if candidates.len() == 1 {
                pending.push_back((
                    candidates[0],
                    execution_uncertain || call.execution_uncertain,
                ));
            } else if candidates.len() > 1 {
                indeterminate_reasons.push(format!(
                    "could not resolve local function call `{}` from {}:{}",
                    call.path.join("::"),
                    definition.location.path.display(),
                    definition.location.line
                ));
            }
        }
    }

    let reachable_functions = definitely_reachable
        .iter()
        .chain(&uncertainly_reachable)
        .map(|index| definitions[*index].location.clone())
        .collect();
    let execution_uncertain_functions = uncertainly_reachable
        .iter()
        .map(|index| definitions[*index].location.clone())
        .collect();

    ReachabilityAnalysis {
        reachable_functions,
        execution_uncertain_functions,
        indeterminate_reasons,
    }
}

fn collect_function_definitions(
    items: &[syn::Item],
    path: &Path,
    module_path: &[String],
    module_imports: &HashMap<Vec<String>, FunctionImports>,
    diverging_functions: &HashSet<String>,
    definitions: &mut Vec<FunctionDefinition>,
) {
    for item in items {
        match item {
            syn::Item::Fn(function) => {
                add_function_definition(
                    function,
                    path,
                    module_path,
                    module_imports.get(module_path).cloned().unwrap_or_default(),
                    diverging_functions,
                    definitions,
                );
            },
            syn::Item::Mod(module) => {
                let Some((_, items)) = &module.content else {
                    continue;
                };
                let mut nested_module_path = module_path.to_vec();
                nested_module_path.push(module.ident.to_string());
                collect_function_definitions(
                    items,
                    path,
                    &nested_module_path,
                    module_imports,
                    diverging_functions,
                    definitions,
                );
            },
            _ => {},
        }
    }
}

fn add_function_definition(
    function: &syn::ItemFn,
    path: &Path,
    module_path: &[String],
    imports: FunctionImports,
    diverging_functions: &HashSet<String>,
    definitions: &mut Vec<FunctionDefinition>,
) {
    let mut calls = Vec::new();
    let parameter_bindings = function
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            syn::FnArg::Typed(argument) => Some(&*argument.pat),
            syn::FnArg::Receiver(_) => None,
        })
        .fold(HashSet::new(), |mut bindings, pattern| {
            collect_pattern_bindings(pattern, &mut bindings);
            bindings
        });
    let generic_path_bindings = function
        .sig
        .generics
        .params
        .iter()
        .filter_map(|parameter| match parameter {
            syn::GenericParam::Type(parameter) => Some(parameter.ident.to_string()),
            syn::GenericParam::Const(_) | syn::GenericParam::Lifetime(_) => None,
        })
        .collect();
    let mut visitor = FunctionCallVisitor {
        calls: &mut calls,
        execution_uncertain_depth: 0,
        diverging_functions,
        import_scopes: vec![imports],
        local_function_scopes: Vec::new(),
        local_value_scopes: vec![parameter_bindings],
        path_prefix_scopes: vec![generic_path_bindings],
    };
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
    calls: &'a mut Vec<FunctionCall>,
    execution_uncertain_depth: usize,
    diverging_functions: &'a HashSet<String>,
    import_scopes: Vec<FunctionImports>,
    local_function_scopes: Vec<HashSet<String>>,
    local_value_scopes: Vec<HashSet<String>>,
    path_prefix_scopes: Vec<HashSet<String>>,
}

impl FunctionCallVisitor<'_> {
    fn visit_execution_uncertain(&mut self, visit: impl FnOnce(&mut Self)) {
        self.execution_uncertain_depth += 1;
        visit(self);
        self.execution_uncertain_depth -= 1;
    }

    fn resolve_call_path(&self, path: &syn::Path) -> Option<Vec<String>> {
        let mut segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        if path.leading_colon.is_some() {
            return None;
        }

        let name = &segments[0];
        if segments.len() == 1
            && (self
                .local_function_scopes
                .iter()
                .rev()
                .any(|scope| scope.contains(name))
                || self
                    .local_value_scopes
                    .iter()
                    .rev()
                    .any(|scope| scope.contains(name)))
        {
            return None;
        }
        if segments.len() > 1
            && self
                .path_prefix_scopes
                .iter()
                .rev()
                .any(|scope| scope.contains(name))
        {
            return None;
        }
        for scope in self.import_scopes.iter().rev() {
            if scope.uncertain_names.contains(name) {
                return None;
            }
            if let Some(imported) = scope.resolved.get(name) {
                let mut resolved = imported.clone();
                resolved.extend(segments.into_iter().skip(1));
                segments = resolved;
                break;
            }
            if scope.has_glob {
                return None;
            }
        }
        Some(segments)
    }

    fn with_local_value_bindings(
        &mut self,
        bindings: HashSet<String>,
        visit: impl FnOnce(&mut Self),
    ) {
        self.local_value_scopes.push(bindings);
        visit(self);
        self.local_value_scopes.pop();
    }

    fn visit_condition_and_collect_bindings(&mut self, expression: &syn::Expr) -> HashSet<String> {
        match expression {
            syn::Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
                let mut bindings = self.visit_condition_and_collect_bindings(&binary.left);
                let visible = bindings.clone();
                let mut right_bindings = HashSet::new();
                self.visit_execution_uncertain(|visitor| {
                    if visible.is_empty() {
                        right_bindings =
                            visitor.visit_condition_and_collect_bindings(&binary.right);
                    } else {
                        visitor.with_local_value_bindings(visible, |visitor| {
                            right_bindings =
                                visitor.visit_condition_and_collect_bindings(&binary.right);
                        });
                    }
                });
                bindings.extend(right_bindings);
                bindings
            },
            syn::Expr::Let(expression) => {
                self.visit_expr(&expression.expr);
                let mut bindings = HashSet::new();
                collect_pattern_bindings(&expression.pat, &mut bindings);
                bindings
            },
            syn::Expr::Group(expression) => {
                self.visit_condition_and_collect_bindings(&expression.expr)
            },
            syn::Expr::Paren(expression) => {
                self.visit_condition_and_collect_bindings(&expression.expr)
            },
            _ => {
                self.visit_expr(expression);
                HashSet::new()
            },
        }
    }
}

impl<'ast> syn::visit::Visit<'ast> for FunctionCallVisitor<'_> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = &*call.func
            && let Some(path) = self.resolve_call_path(&function.path)
        {
            self.calls.push(FunctionCall {
                path,
                execution_uncertain: self.execution_uncertain_depth > 0
                    || has_conditional_attr(&call.attrs),
            });
        }
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        let bindings = self.visit_condition_and_collect_bindings(&expression.cond);
        self.visit_execution_uncertain(|visitor| {
            visitor.with_local_value_bindings(bindings, |visitor| {
                visitor.visit_block(&expression.then_branch);
            });
        });
        if let Some((_, else_branch)) = &expression.else_branch {
            self.visit_execution_uncertain(|visitor| {
                visitor.visit_expr(else_branch);
            });
        }
    }

    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        self.visit_expr(&expression.expr);
        for arm in &expression.arms {
            let (pattern, guard) = match &arm.pat {
                syn::Pat::Guard(guard) => (&*guard.pat, Some(&*guard.guard)),
                pattern => (pattern, None),
            };
            let mut bindings = HashSet::new();
            collect_pattern_bindings(pattern, &mut bindings);
            self.visit_execution_uncertain(|visitor| {
                visitor.with_local_value_bindings(bindings, |visitor| {
                    if let Some(guard) = guard {
                        visitor.visit_expr(guard);
                    }
                    visitor.visit_expr(&arm.body);
                });
            });
        }
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        let bindings = self.visit_condition_and_collect_bindings(&expression.cond);
        self.visit_execution_uncertain(|visitor| {
            visitor.with_local_value_bindings(bindings, |visitor| {
                visitor.visit_block(&expression.body);
            });
        });
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.visit_expr(&expression.expr);
        let mut bindings = HashSet::new();
        collect_pattern_bindings(&expression.pat, &mut bindings);
        self.visit_execution_uncertain(|visitor| {
            visitor.with_local_value_bindings(bindings, |visitor| {
                visitor.visit_block(&expression.body);
            });
        });
    }

    fn visit_expr_binary(&mut self, expression: &'ast syn::ExprBinary) {
        self.visit_expr(&expression.left);
        if matches!(expression.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
            self.visit_execution_uncertain(|visitor| {
                visitor.visit_expr(&expression.right);
            });
        } else {
            self.visit_expr(&expression.right);
        }
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let imports = block
            .stmts
            .iter()
            .filter_map(|statement| match statement {
                syn::Stmt::Item(syn::Item::Use(item)) => Some(item),
                _ => None,
            })
            .fold(FunctionImports::default(), |mut imports, item| {
                collect_function_imports(item, &mut imports);
                imports
            });
        let local_functions = block
            .stmts
            .iter()
            .filter_map(|statement| match statement {
                syn::Stmt::Item(syn::Item::Fn(function)) => Some(function.sig.ident.to_string()),
                _ => None,
            })
            .collect();
        let local_value_items = block
            .stmts
            .iter()
            .filter_map(|statement| match statement {
                syn::Stmt::Item(item) => callable_value_item_name(item),
                _ => None,
            })
            .collect();
        let path_prefix_items = block
            .stmts
            .iter()
            .filter_map(|statement| match statement {
                syn::Stmt::Item(item) => path_prefix_item_name(item),
                _ => None,
            })
            .collect();
        self.import_scopes.push(imports);
        self.local_function_scopes.push(local_functions);
        self.local_value_scopes.push(local_value_items);
        self.path_prefix_scopes.push(path_prefix_items);
        let mut following_execution_uncertain = false;
        for statement in &block.stmts {
            if following_execution_uncertain {
                self.visit_execution_uncertain(|visitor| visitor.visit_stmt(statement));
            } else {
                self.visit_stmt(statement);
            }
            if statement_unconditionally_terminates(statement) {
                break;
            }
            following_execution_uncertain |=
                statement_may_skip_following(statement, self.diverging_functions);
            if let syn::Stmt::Local(local) = statement {
                collect_pattern_bindings(
                    &local.pat,
                    self.local_value_scopes
                        .last_mut()
                        .expect("block value scope was just pushed"),
                );
            }
        }
        self.path_prefix_scopes.pop();
        self.local_value_scopes.pop();
        self.local_function_scopes.pop();
        self.import_scopes.pop();
    }

    fn visit_item_fn(&mut self, _function: &'ast syn::ItemFn) {}

    fn visit_expr_closure(&mut self, _closure: &'ast syn::ExprClosure) {}

    fn visit_expr_async(&mut self, _expression: &'ast syn::ExprAsync) {}
}

fn collect_pattern_bindings(pattern: &syn::Pat, bindings: &mut HashSet<String>) {
    struct Visitor<'a> {
        bindings: &'a mut HashSet<String>,
    }

    impl<'ast> syn::visit::Visit<'ast> for Visitor<'_> {
        fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
            self.bindings.insert(pattern.ident.to_string());
            syn::visit::visit_pat_ident(self, pattern);
        }
    }

    Visitor { bindings }.visit_pat(pattern);
}

fn callable_value_item_name(item: &syn::Item) -> Option<String> {
    match item {
        syn::Item::Const(item) => Some(item.ident.to_string()),
        syn::Item::Static(item) => Some(item.ident.to_string()),
        syn::Item::Struct(item) if !matches!(item.fields, syn::Fields::Named(_)) => {
            Some(item.ident.to_string())
        },
        _ => None,
    }
}

fn path_prefix_item_name(item: &syn::Item) -> Option<String> {
    let ident = match item {
        syn::Item::Enum(item) => &item.ident,
        syn::Item::ExternCrate(item) => item
            .rename
            .as_ref()
            .map_or(&item.ident, |(_, rename)| rename),
        syn::Item::Mod(item) => &item.ident,
        syn::Item::Struct(item) => &item.ident,
        syn::Item::Trait(item) => &item.ident,
        syn::Item::TraitAlias(item) => &item.ident,
        syn::Item::Type(item) => &item.ident,
        syn::Item::Union(item) => &item.ident,
        _ => return None,
    };
    Some(ident.to_string())
}

#[derive(Clone, Debug, Default)]
struct FunctionImports {
    resolved: HashMap<String, Vec<String>>,
    uncertain_names: HashSet<String>,
    has_glob: bool,
}

fn collect_module_function_imports(
    items: &[syn::Item],
    module_path: &[String],
    imports_by_module: &mut HashMap<Vec<String>, FunctionImports>,
) {
    let imports = imports_by_module.entry(module_path.to_vec()).or_default();
    for item in items {
        if let syn::Item::Use(item) = item {
            collect_function_imports(item, imports);
        }
    }

    for item in items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        let Some((_, nested_items)) = &module.content else {
            continue;
        };
        let mut nested_module_path = module_path.to_vec();
        nested_module_path.push(module.ident.to_string());
        collect_module_function_imports(nested_items, &nested_module_path, imports_by_module);
    }
}

fn collect_function_imports(item: &syn::ItemUse, imports: &mut FunctionImports) {
    let uncertain = item.leading_colon.is_some() || has_conditional_attr(&item.attrs);
    collect_function_use_tree(&item.tree, &mut Vec::new(), imports, uncertain);
}

fn collect_function_use_tree(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    imports: &mut FunctionImports,
    uncertain: bool,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_function_use_tree(&path.tree, prefix, imports, uncertain);
            prefix.pop();
        },
        syn::UseTree::Name(name) if name.ident == "self" => {
            if let Some(bound) = prefix.last() {
                record_function_import(imports, bound.clone(), prefix.clone(), uncertain);
            }
        },
        syn::UseTree::Name(name) => {
            let bound = name.ident.to_string();
            let mut path = prefix.clone();
            path.push(bound.clone());
            record_function_import(imports, bound, path, uncertain);
        },
        syn::UseTree::Rename(rename) => {
            let mut path = prefix.clone();
            if rename.ident != "self" {
                path.push(rename.ident.to_string());
            }
            record_function_import(imports, rename.rename.to_string(), path, uncertain);
        },
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                collect_function_use_tree(tree, prefix, imports, uncertain);
            }
        },
        syn::UseTree::Glob(_) => imports.has_glob = true,
    }
}

fn record_function_import(
    imports: &mut FunctionImports,
    bound: String,
    path: Vec<String>,
    uncertain: bool,
) {
    if uncertain {
        imports.uncertain_names.insert(bound);
    } else {
        imports.resolved.insert(bound, path);
    }
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
    } else {
        while relative.first().is_some_and(|segment| segment == "super") {
            if module_path.pop().is_none() {
                return Vec::new();
            }
            relative = &relative[1..];
        }
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
    shadowed: bool,
    expected_root_shadowed: bool,
    unexpected_manager_root: bool,
}

impl ImportResolution {
    fn merge(&mut self, other: Self) {
        self.verified |= other.verified;
        self.uncertain |= other.uncertain;
        self.shadowed |= other.shadowed;
        self.expected_root_shadowed |= other.expected_root_shadowed;
        self.unexpected_manager_root |= other.unexpected_manager_root;
    }
}

struct EvidenceVisitor<'a> {
    target: Option<SourceTarget<'a>>,
    current_file: &'a Path,
    conditional_depth: usize,
    current_function: Option<FunctionLocation>,
    execution_uncertain_depth: usize,
    diverging_functions: HashSet<String>,
    evidences: Vec<MatchedEvidence>,
    includes: Vec<IncludeSource>,
    scopes: Vec<ImportResolution>,
    indeterminate_reasons: Vec<String>,
}

impl<'a> EvidenceVisitor<'a> {
    fn new(
        target: Option<SourceTarget<'a>>,
        current_file: &'a Path,
        conditional: bool,
        diverging_functions: HashSet<String>,
    ) -> Self {
        Self {
            target,
            current_file,
            conditional_depth: usize::from(conditional),
            current_function: None,
            execution_uncertain_depth: 0,
            diverging_functions,
            evidences: Vec::new(),
            includes: Vec::new(),
            scopes: Vec::new(),
            indeterminate_reasons: Vec::new(),
        }
    }

    fn record(&mut self, path: &syn::Path, span: Span, conditional: bool) {
        let (verified, unexpected_manager_root) = if path.segments.len() > 1 {
            let verified = self.target.is_some_and(|target| {
                target.is_expected_path(path)
                    && (path.leading_colon.is_some()
                        || !self
                            .scopes
                            .iter()
                            .rev()
                            .any(|scope| scope.expected_root_shadowed))
            });
            let unexpected_manager_root = self.target.is_some_and(|target| {
                path.segments.first().is_some_and(|segment| {
                    target.is_unexpected_manager_root(&segment.ident.to_string())
                })
            });
            (verified, unexpected_manager_root)
        } else {
            self.scopes
                .iter()
                .rev()
                .find_map(|scope| {
                    if scope.shadowed || scope.uncertain {
                        Some((false, false))
                    } else if scope.verified {
                        Some((true, false))
                    } else if scope.unexpected_manager_root {
                        Some((false, true))
                    } else {
                        None
                    }
                })
                .unwrap_or((false, false))
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
            unexpected_manager_root,
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

    fn visit_execution_uncertain(&mut self, visit: impl FnOnce(&mut Self)) {
        self.execution_uncertain_depth += 1;
        visit(self);
        self.execution_uncertain_depth -= 1;
    }

    fn is_target_macro(&self, invocation: &syn::Macro) -> bool {
        self.target
            .filter(|target| matches!(target, SourceTarget::Macro(_, _)))
            .is_some_and(|target| {
                invocation
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == target.name())
            })
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
        let direct_target_call = if let (Some(target), syn::Expr::Path(function)) =
            (self.target, &*call.func)
            && let Some(target) = target.call_name()
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
            true
        } else {
            false
        };

        if direct_target_call {
            for argument in &call.args {
                self.visit_expr(argument);
            }
        } else {
            syn::visit::visit_expr_call(self, call);
        }
    }

    fn visit_expr_path(&mut self, expression: &'ast syn::ExprPath) {
        if let Some(target) = self.target.and_then(SourceTarget::call_name)
            && expression
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == target)
        {
            self.indeterminate_reasons.push(format!(
                "opaque reference to `{target}` at {}:{}",
                self.current_file.display(),
                expression.path.span().start().line
            ));
        }
        syn::visit::visit_expr_path(self, expression);
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
        if let Some(target @ SourceTarget::Macro(_, _)) = self.target
            && invocation
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == target.name())
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
        self.visit_execution_uncertain(|visitor| {
            syn::visit::visit_expr_closure(visitor, closure);
        });
    }

    fn visit_expr_async(&mut self, expression: &'ast syn::ExprAsync) {
        self.visit_execution_uncertain(|visitor| {
            syn::visit::visit_expr_async(visitor, expression);
        });
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        self.visit_with_attributes(&expression.attrs, |visitor| {
            visitor.visit_expr(&expression.cond);
            visitor.visit_execution_uncertain(|visitor| {
                visitor.visit_block(&expression.then_branch);
            });
            if let Some((_, else_branch)) = &expression.else_branch {
                visitor.visit_execution_uncertain(|visitor| {
                    visitor.visit_expr(else_branch);
                });
            }
        });
    }

    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        self.visit_with_attributes(&expression.attrs, |visitor| {
            visitor.visit_expr(&expression.expr);
            for arm in &expression.arms {
                visitor.visit_execution_uncertain(|visitor| {
                    visitor.visit_pat(&arm.pat);
                    visitor.visit_expr(&arm.body);
                });
            }
        });
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        self.visit_with_attributes(&expression.attrs, |visitor| {
            visitor.visit_expr(&expression.cond);
            visitor.visit_execution_uncertain(|visitor| {
                visitor.visit_block(&expression.body);
            });
        });
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.visit_with_attributes(&expression.attrs, |visitor| {
            visitor.visit_expr(&expression.expr);
            visitor.visit_execution_uncertain(|visitor| {
                visitor.visit_block(&expression.body);
            });
        });
    }

    fn visit_expr_binary(&mut self, expression: &'ast syn::ExprBinary) {
        self.visit_with_attributes(&expression.attrs, |visitor| {
            visitor.visit_expr(&expression.left);
            if matches!(expression.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
                visitor.visit_execution_uncertain(|visitor| {
                    visitor.visit_expr(&expression.right);
                });
            } else {
                visitor.visit_expr(&expression.right);
            }
        });
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
        let mut following_execution_uncertain = false;
        for statement in &block.stmts {
            if following_execution_uncertain {
                self.visit_execution_uncertain(|visitor| visitor.visit_stmt(statement));
            } else {
                self.visit_stmt(statement);
            }
            if self.target.is_some_and(SourceTarget::is_call)
                && statement_unconditionally_terminates(statement)
            {
                break;
            }
            following_execution_uncertain |=
                statement_may_skip_following(statement, &self.diverging_functions);
            if let syn::Stmt::Local(local) = statement
                && local_shadows_target(&local.pat, self.target)
            {
                self.scopes
                    .last_mut()
                    .expect("block scope was just pushed")
                    .shadowed = true;
            }
        }
        self.scopes.pop();
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        self.visit_with_attributes(&item.attrs, |visitor| {
            if item.mac.path.is_ident("macro_rules") {
                if token_stream_contains_macro_invocation(&item.mac.tokens, "include") {
                    visitor.indeterminate_reasons.push(format!(
                        "macro wrapper involving `include!` at {}:{}",
                        visitor.current_file.display(),
                        item.mac.path.span().start().line
                    ));
                }
                if let Some(target) = visitor.target
                    && token_stream_contains_ident(&item.mac.tokens, target.name())
                {
                    visitor.indeterminate_reasons.push(format!(
                        "macro wrapper involving `{}` at {}:{}",
                        target.name(),
                        visitor.current_file.display(),
                        item.mac.path.span().start().line
                    ));
                }
            } else if !item.mac.path.is_ident("include") && !visitor.is_target_macro(&item.mac) {
                visitor.indeterminate_reasons.push(format!(
                    "opaque item macro expansion at {}:{}",
                    visitor.current_file.display(),
                    item.mac.path.span().start().line
                ));
            }
            syn::visit::visit_item_macro(visitor, item);
        });
    }

    fn visit_expr_macro(&mut self, expression: &'ast syn::ExprMacro) {
        self.visit_with_attributes(&expression.attrs, |visitor| {
            if !expression.mac.path.is_ident("include") && !visitor.is_target_macro(&expression.mac)
            {
                visitor.indeterminate_reasons.push(format!(
                    "opaque expression macro expansion at {}:{}",
                    visitor.current_file.display(),
                    expression.mac.path.span().start().line
                ));
            }
            syn::visit::visit_expr_macro(visitor, expression);
        });
    }

    fn visit_stmt_macro(&mut self, statement: &'ast syn::StmtMacro) {
        self.visit_with_attributes(&statement.attrs, |visitor| {
            if !statement.mac.path.is_ident("include") && !visitor.is_target_macro(&statement.mac) {
                visitor.indeterminate_reasons.push(format!(
                    "opaque statement macro expansion at {}:{}",
                    visitor.current_file.display(),
                    statement.mac.path.span().start().line
                ));
            }
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

fn imports_for_items(items: &[syn::Item], target: Option<SourceTarget<'_>>) -> ImportResolution {
    items
        .iter()
        .fold(ImportResolution::default(), |mut found, item| {
            match item {
                syn::Item::Use(item) => found.merge(import_resolution(item, target)),
                item if item_shadows_target(item, target) => found.shadowed = true,
                _ => {},
            }
            found.expected_root_shadowed |= item_shadows_expected_root(item, target);
            found
        })
}

fn imports_for_statements(
    statements: &[syn::Stmt],
    target: Option<SourceTarget<'_>>,
) -> ImportResolution {
    statements
        .iter()
        .fold(ImportResolution::default(), |mut found, statement| {
            if let syn::Stmt::Item(item) = statement {
                match item {
                    syn::Item::Use(item) => found.merge(import_resolution(item, target)),
                    item if item_shadows_target(item, target) => found.shadowed = true,
                    _ => {},
                }
                found.expected_root_shadowed |= item_shadows_expected_root(item, target);
            }
            found
        })
}

fn local_shadows_target(pattern: &syn::Pat, target: Option<SourceTarget<'_>>) -> bool {
    let Some(target) = target.and_then(SourceTarget::call_name) else {
        return false;
    };
    let mut visitor = PatternBindingVisitor {
        target,
        found: false,
    };
    visitor.visit_pat(pattern);
    visitor.found
}

struct PatternBindingVisitor<'a> {
    target: &'a str,
    found: bool,
}

impl<'ast> syn::visit::Visit<'ast> for PatternBindingVisitor<'_> {
    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.found |= pattern.ident == self.target;
        syn::visit::visit_pat_ident(self, pattern);
    }
}

fn statement_unconditionally_terminates(statement: &syn::Stmt) -> bool {
    match statement {
        syn::Stmt::Local(local) => local
            .init
            .as_ref()
            .is_some_and(|init| expression_unconditionally_terminates(&init.expr)),
        syn::Stmt::Expr(expression, _) => expression_unconditionally_terminates(expression),
        syn::Stmt::Item(_) | syn::Stmt::Macro(_) => false,
    }
}

fn statement_may_skip_following(
    statement: &syn::Stmt,
    diverging_functions: &HashSet<String>,
) -> bool {
    if let syn::Stmt::Expr(syn::Expr::Loop(expression), _) = statement
        && !loop_definitely_exits(expression)
    {
        return true;
    }

    let mut visitor = FollowingStatementExitVisitor {
        found: false,
        nested_loop_depth: 0,
        nested_try_block_depth: 0,
        diverging_functions,
    };
    visitor.visit_stmt(statement);
    visitor.found
}

fn loop_definitely_exits(expression: &syn::ExprLoop) -> bool {
    let Some(syn::Stmt::Expr(syn::Expr::Break(exit), _)) = expression.body.stmts.first() else {
        return false;
    };
    if exit.expr.is_some() {
        return false;
    }
    exit.label.as_ref().is_none_or(|label| {
        expression
            .label
            .as_ref()
            .is_some_and(|loop_label| label.ident == loop_label.name.ident)
    })
}

struct FollowingStatementExitVisitor<'a> {
    found: bool,
    nested_loop_depth: usize,
    nested_try_block_depth: usize,
    diverging_functions: &'a HashSet<String>,
}

impl<'ast> syn::visit::Visit<'ast> for FollowingStatementExitVisitor<'_> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        self.found |= call_is_known_to_diverge(call, self.diverging_functions);
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_macro(&mut self, invocation: &'ast syn::Macro) {
        if !invocation.path.is_ident("include") {
            self.found = true;
        }
    }

    fn visit_expr_return(&mut self, expression: &'ast syn::ExprReturn) {
        self.found = true;
        syn::visit::visit_expr_return(self, expression);
    }

    fn visit_expr_break(&mut self, expression: &'ast syn::ExprBreak) {
        if self.nested_loop_depth == 0 || expression.label.is_some() {
            self.found = true;
        }
        syn::visit::visit_expr_break(self, expression);
    }

    fn visit_expr_continue(&mut self, expression: &'ast syn::ExprContinue) {
        if self.nested_loop_depth == 0 || expression.label.is_some() {
            self.found = true;
        }
        syn::visit::visit_expr_continue(self, expression);
    }

    fn visit_expr_try(&mut self, expression: &'ast syn::ExprTry) {
        if self.nested_try_block_depth == 0 {
            self.found = true;
        }
        syn::visit::visit_expr_try(self, expression);
    }

    fn visit_expr_try_block(&mut self, expression: &'ast syn::ExprTryBlock) {
        self.nested_try_block_depth += 1;
        syn::visit::visit_expr_try_block(self, expression);
        self.nested_try_block_depth -= 1;
    }

    fn visit_expr_loop(&mut self, expression: &'ast syn::ExprLoop) {
        self.found |= !loop_definitely_exits(expression);
        self.nested_loop_depth += 1;
        syn::visit::visit_expr_loop(self, expression);
        self.nested_loop_depth -= 1;
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        self.nested_loop_depth += 1;
        syn::visit::visit_expr_while(self, expression);
        self.nested_loop_depth -= 1;
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.nested_loop_depth += 1;
        syn::visit::visit_expr_for_loop(self, expression);
        self.nested_loop_depth -= 1;
    }

    fn visit_item(&mut self, _item: &'ast syn::Item) {}

    fn visit_expr_closure(&mut self, _closure: &'ast syn::ExprClosure) {}

    fn visit_expr_async(&mut self, _expression: &'ast syn::ExprAsync) {}
}

fn call_is_known_to_diverge(call: &syn::ExprCall, diverging_functions: &HashSet<String>) -> bool {
    let syn::Expr::Path(function) = &*call.func else {
        return false;
    };
    let segments = function
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();

    matches!(
        segments.as_slice(),
        [standard, process, function]
            if standard == "std"
                && process == "process"
                && matches!(function.as_str(), "exit" | "abort")
    ) || matches!(
        segments.as_slice(),
        [standard, panic, function]
            if standard == "std" && panic == "panic" && function == "resume_unwind"
    ) || segments
        .last()
        .is_some_and(|name| diverging_functions.contains(name))
}

fn diverging_function_names(file: &syn::File) -> HashSet<String> {
    #[derive(Default)]
    struct Visitor {
        names: HashSet<String>,
    }

    impl<'ast> syn::visit::Visit<'ast> for Visitor {
        fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
            if matches!(
                &function.sig.output,
                syn::ReturnType::Type(_, output) if matches!(&**output, syn::Type::Never(_))
            ) {
                self.names.insert(function.sig.ident.to_string());
            }
            syn::visit::visit_item_fn(self, function);
        }
    }

    let mut visitor = Visitor::default();
    visitor.visit_file(file);
    visitor.names
}

fn expression_unconditionally_terminates(expression: &syn::Expr) -> bool {
    match expression {
        syn::Expr::Array(array) => expressions_unconditionally_terminate(&array.elems),
        syn::Expr::Assign(assign) => {
            expression_unconditionally_terminates(&assign.left)
                || expression_unconditionally_terminates(&assign.right)
        },
        syn::Expr::Await(await_) => expression_unconditionally_terminates(&await_.base),
        syn::Expr::Binary(binary) => {
            expression_unconditionally_terminates(&binary.left)
                || (!matches!(binary.op, syn::BinOp::And(_) | syn::BinOp::Or(_))
                    && expression_unconditionally_terminates(&binary.right))
        },
        syn::Expr::Break(_) | syn::Expr::Continue(_) | syn::Expr::Return(_) => true,
        syn::Expr::Block(block) => block_unconditionally_terminates(&block.block),
        syn::Expr::Call(call) => {
            expression_unconditionally_terminates(&call.func)
                || expressions_unconditionally_terminate(&call.args)
        },
        syn::Expr::Cast(cast) => expression_unconditionally_terminates(&cast.expr),
        syn::Expr::Field(field) => expression_unconditionally_terminates(&field.base),
        syn::Expr::ForLoop(for_loop) => expression_unconditionally_terminates(&for_loop.expr),
        syn::Expr::Group(group) => expression_unconditionally_terminates(&group.expr),
        syn::Expr::If(if_) => expression_unconditionally_terminates(&if_.cond),
        syn::Expr::Index(index) => {
            expression_unconditionally_terminates(&index.expr)
                || expression_unconditionally_terminates(&index.index)
        },
        syn::Expr::Let(let_) => expression_unconditionally_terminates(&let_.expr),
        syn::Expr::Loop(expression) => !loop_can_reach_following(expression),
        syn::Expr::Match(match_) => expression_unconditionally_terminates(&match_.expr),
        syn::Expr::MethodCall(call) => {
            expression_unconditionally_terminates(&call.receiver)
                || expressions_unconditionally_terminate(&call.args)
        },
        syn::Expr::Paren(paren) => expression_unconditionally_terminates(&paren.expr),
        syn::Expr::Range(range) => {
            range
                .start
                .as_deref()
                .is_some_and(expression_unconditionally_terminates)
                || range
                    .end
                    .as_deref()
                    .is_some_and(expression_unconditionally_terminates)
        },
        syn::Expr::RawAddr(address) => expression_unconditionally_terminates(&address.expr),
        syn::Expr::Reference(reference) => expression_unconditionally_terminates(&reference.expr),
        syn::Expr::Repeat(repeat) => expression_unconditionally_terminates(&repeat.expr),
        syn::Expr::Struct(struct_) => {
            struct_
                .fields
                .iter()
                .any(|field| expression_unconditionally_terminates(&field.expr))
                || struct_
                    .rest
                    .as_deref()
                    .is_some_and(expression_unconditionally_terminates)
        },
        syn::Expr::Try(try_) => expression_unconditionally_terminates(&try_.expr),
        syn::Expr::TryBlock(try_block) => block_unconditionally_terminates(&try_block.block),
        syn::Expr::Tuple(tuple) => expressions_unconditionally_terminate(&tuple.elems),
        syn::Expr::Unary(unary) => expression_unconditionally_terminates(&unary.expr),
        syn::Expr::Unsafe(unsafe_) => block_unconditionally_terminates(&unsafe_.block),
        syn::Expr::While(while_) => expression_unconditionally_terminates(&while_.cond),
        syn::Expr::Yield(yield_) => yield_
            .expr
            .as_deref()
            .is_some_and(expression_unconditionally_terminates),
        _ => false,
    }
}

fn block_unconditionally_terminates(block: &syn::Block) -> bool {
    block.stmts.iter().any(statement_unconditionally_terminates)
}

fn expressions_unconditionally_terminate(
    expressions: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
) -> bool {
    expressions
        .iter()
        .any(expression_unconditionally_terminates)
}

fn loop_can_reach_following(expression: &syn::ExprLoop) -> bool {
    let mut visitor = LoopExitVisitor {
        loop_label: expression
            .label
            .as_ref()
            .map(|label| label.name.ident.to_string()),
        nested_loop_depth: 0,
        found: false,
    };
    visitor.visit_block(&expression.body);
    visitor.found
}

struct LoopExitVisitor {
    loop_label: Option<String>,
    nested_loop_depth: usize,
    found: bool,
}

impl<'ast> syn::visit::Visit<'ast> for LoopExitVisitor {
    fn visit_expr_break(&mut self, expression: &'ast syn::ExprBreak) {
        let exits_loop = expression
            .label
            .as_ref()
            .map_or(self.nested_loop_depth == 0, |label| {
                self.loop_label
                    .as_deref()
                    .is_some_and(|loop_label| label.ident == loop_label)
            });
        self.found |= exits_loop;
    }

    fn visit_expr_loop(&mut self, expression: &'ast syn::ExprLoop) {
        self.nested_loop_depth += 1;
        syn::visit::visit_expr_loop(self, expression);
        self.nested_loop_depth -= 1;
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        self.nested_loop_depth += 1;
        syn::visit::visit_expr_while(self, expression);
        self.nested_loop_depth -= 1;
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.nested_loop_depth += 1;
        syn::visit::visit_expr_for_loop(self, expression);
        self.nested_loop_depth -= 1;
    }

    fn visit_item_fn(&mut self, _function: &'ast syn::ItemFn) {}

    fn visit_expr_closure(&mut self, _closure: &'ast syn::ExprClosure) {}

    fn visit_expr_async(&mut self, _expression: &'ast syn::ExprAsync) {}
}

fn item_shadows_target(item: &syn::Item, target: Option<SourceTarget<'_>>) -> bool {
    let Some(target) = target.and_then(SourceTarget::call_name) else {
        return false;
    };
    let ident = match item {
        syn::Item::Const(item) => Some(&item.ident),
        syn::Item::Enum(item) => Some(&item.ident),
        syn::Item::Fn(item) => Some(&item.sig.ident),
        syn::Item::Static(item) => Some(&item.ident),
        syn::Item::Struct(item) => Some(&item.ident),
        syn::Item::Union(item) => Some(&item.ident),
        _ => None,
    };
    ident.is_some_and(|ident| ident == target)
}

fn item_shadows_expected_root(item: &syn::Item, target: Option<SourceTarget<'_>>) -> bool {
    let Some(target) = target else {
        return false;
    };
    let ident = match item {
        syn::Item::Enum(item) => Some(&item.ident),
        syn::Item::ExternCrate(item) => {
            let bound = item
                .rename
                .as_ref()
                .map_or(&item.ident, |(_, rename)| rename);
            return target.is_expected_root(&bound.to_string())
                && !target.is_expected_root(&item.ident.to_string());
        },
        syn::Item::Mod(item) => Some(&item.ident),
        syn::Item::Struct(item) => Some(&item.ident),
        syn::Item::Trait(item) => Some(&item.ident),
        syn::Item::TraitAlias(item) => Some(&item.ident),
        syn::Item::Type(item) => Some(&item.ident),
        syn::Item::Union(item) => Some(&item.ident),
        _ => None,
    };
    ident.is_some_and(|ident| target.is_expected_root(&ident.to_string()))
}

fn import_resolution(item: &syn::ItemUse, target: Option<SourceTarget<'_>>) -> ImportResolution {
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
    target: SourceTarget<'_>,
    resolution: &mut ImportResolution,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.clone());
            inspect_use_tree(&path.tree, prefix, target, resolution);
            prefix.pop();
        },
        syn::UseTree::Name(name) => {
            if target.is_expected_root(&name.ident.to_string()) && !prefix.is_empty() {
                resolution.expected_root_shadowed = true;
            }
            if name.ident == "self"
                && prefix
                    .last()
                    .is_some_and(|ident| target.is_expected_root(&ident.to_string()))
                && prefix.len() > 1
            {
                resolution.expected_root_shadowed = true;
            }
            prefix.push(name.ident.clone());
            if name.ident == target.name() {
                let path = syn::Path {
                    leading_colon: None,
                    segments: prefix.iter().cloned().map(syn::PathSegment::from).collect(),
                };
                if target.is_expected_path(&path) {
                    resolution.verified = true;
                } else if path.segments.first().is_some_and(|segment| {
                    target.is_unexpected_manager_root(&segment.ident.to_string())
                }) {
                    resolution.unexpected_manager_root = true;
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
            if target.is_expected_root(&rename.rename.to_string())
                && (!prefix.is_empty() || !target.is_expected_root(&rename.ident.to_string()))
            {
                resolution.expected_root_shadowed = true;
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

fn token_stream_contains_macro_invocation(tokens: &proc_macro2::TokenStream, target: &str) -> bool {
    let tokens = tokens.clone().into_iter().collect::<Vec<_>>();
    if tokens.windows(2).any(|tokens| {
        matches!(&tokens[0], proc_macro2::TokenTree::Ident(ident) if ident == target)
            && matches!(&tokens[1], proc_macro2::TokenTree::Punct(punct) if punct.as_char() == '!')
    }) {
        return true;
    }

    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Group(group) => {
            token_stream_contains_macro_invocation(&group.stream(), target)
        },
        proc_macro2::TokenTree::Ident(_)
        | proc_macro2::TokenTree::Punct(_)
        | proc_macro2::TokenTree::Literal(_) => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures as tempfile;
    use std::fs;

    fn inspect_fixture(
        files: &[(&str, &str)],
        entry: &str,
        target: SourceTarget<'_>,
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

    fn inspect_fixture_with_roots(
        files: &[(&str, &str)],
        entry: &str,
        target: &'static str,
        expected_roots: &[&str],
    ) -> InspectionOutcome {
        let expected_roots = expected_roots
            .iter()
            .map(|root| (*root).to_string())
            .collect::<Vec<_>>();
        inspect_fixture(
            files,
            entry,
            SourceTarget::Macro(target, Some(&expected_roots)),
        )
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
    fn build_helper_wrappers_imported_from_modules_are_reachable() {
        for import_and_call in [
            "use helper::setup; fn main() { setup(); }",
            "use helper::setup as configure; fn main() { configure(); }",
            "fn main() { use helper::setup as configure; configure(); }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[
                        ("build.rs", &format!("mod helper; {import_and_call}")),
                        (
                            "helper.rs",
                            "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
                        ),
                    ],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Found(_)
            ));
        }
    }

    #[test]
    fn qualified_calls_through_imported_module_bindings_are_reachable() {
        for import_and_call in [
            "use crate::helper as h; pub fn configure() { h::setup(); }",
            "use crate::helper; pub fn configure() { helper::setup(); }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[
                        (
                            "build.rs",
                            "mod helper; mod nested; fn main() { nested::configure(); }",
                        ),
                        (
                            "helper.rs",
                            "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
                        ),
                        ("nested.rs", import_and_call),
                    ],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Found(_)
            ));
        }
    }

    #[test]
    fn path_namespace_bindings_shadow_qualified_import_prefixes() {
        for (main_call, nested_source) in [
            (
                "nested::configure();",
                concat!(
                    "use crate::helper as h; ",
                    "pub fn configure() { ",
                    "struct h; impl h { fn setup() {} } h::setup(); ",
                    "}",
                ),
            ),
            (
                "nested::configure();",
                concat!(
                    "use crate::helper as h; ",
                    "pub fn configure() { ",
                    "h::setup(); struct h; impl h { fn setup() {} } ",
                    "}",
                ),
            ),
            (
                "nested::configure();",
                concat!(
                    "use crate::helper as h; ",
                    "pub fn configure() { ",
                    "mod h { pub fn setup() {} } h::setup(); ",
                    "}",
                ),
            ),
            (
                "nested::configure();",
                concat!(
                    "use crate::helper as h; ",
                    "struct Local; impl Local { fn setup() {} } ",
                    "pub fn configure() { type h = Local; h::setup(); }",
                ),
            ),
            (
                "nested::configure::<nested::Local>();",
                concat!(
                    "use crate::helper as h; ",
                    "pub trait Setup { fn setup(); } ",
                    "pub struct Local; impl Setup for Local { fn setup() {} } ",
                    "pub fn configure<h: Setup>() { h::setup(); }",
                ),
            ),
        ] {
            let outcome = inspect_fixture(
                &[
                    (
                        "build.rs",
                        &format!("mod helper; mod nested; fn main() {{ {main_call} }}"),
                    ),
                    (
                        "helper.rs",
                        "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
                    ),
                    ("nested.rs", nested_source),
                ],
                "build.rs",
                SourceTarget::Call("track_i18n_assets"),
            );

            assert!(matches!(outcome, InspectionOutcome::Indeterminate(_)));
        }
    }

    #[test]
    fn value_namespace_bindings_do_not_shadow_qualified_import_prefixes() {
        for (main_call, nested_source) in [
            (
                "nested::configure();",
                "use crate::helper as h; pub fn configure() { let h = (); h::setup(); }",
            ),
            (
                "nested::configure::<0>();",
                "use crate::helper as h; pub fn configure<const h: usize>() { h::setup(); }",
            ),
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[
                        (
                            "build.rs",
                            &format!("mod helper; mod nested; fn main() {{ {main_call} }}"),
                        ),
                        (
                            "helper.rs",
                            "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
                        ),
                        ("nested.rs", nested_source),
                    ],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Found(_)
            ));
        }
    }

    #[test]
    fn grouped_self_aliases_and_repeated_super_imports_are_reachable() {
        for (build_source, outer_source) in [
            (
                "mod helper; mod outer; fn main() { outer::configure(); }",
                "use crate::helper::{self as h}; pub fn configure() { h::setup(); }",
            ),
            (
                "mod helper; mod outer; fn main() { outer::nested::configure(); }",
                "pub mod nested { use super::super::helper::setup; pub fn configure() { setup(); } }",
            ),
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[
                        ("build.rs", build_source),
                        (
                            "helper.rs",
                            "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
                        ),
                        ("outer.rs", outer_source),
                    ],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Found(_)
            ));
        }
    }

    #[test]
    fn leading_absolute_calls_do_not_resolve_to_local_modules() {
        let outcome = inspect_fixture(
            &[
                ("build.rs", "mod helper; fn main() { ::helper::setup(); }"),
                (
                    "helper.rs",
                    "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
                ),
            ],
            "build.rs",
            SourceTarget::Call("track_i18n_assets"),
        );

        assert!(matches!(outcome, InspectionOutcome::Indeterminate(_)));
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
                    SourceTarget::Macro("define_i18n_module", None)
                ),
                InspectionOutcome::Found(_)
            ));
        }
    }

    #[test]
    fn manager_macros_must_match_declared_dependency_roots() {
        for source in [
            "es_fluent_manager_bevy::define_i18n_module!();",
            "use es_fluent_manager_bevy::define_i18n_module; define_i18n_module!();",
            "es_fluent_manager_embedded::define_i18n_module!(); es_fluent_manager_bevy::define_i18n_module!();",
        ] {
            assert_eq!(
                inspect_fixture_with_roots(
                    &[("lib.rs", source)],
                    "lib.rs",
                    "define_i18n_module",
                    &["es_fluent_manager_embedded"],
                ),
                InspectionOutcome::NotFound
            );
        }

        assert_eq!(
            inspect_fixture_with_roots(
                &[(
                    "lib.rs",
                    "es_fluent_manager_embedded::define_i18n_module!();"
                )],
                "lib.rs",
                "define_i18n_module",
                &[],
            ),
            InspectionOutcome::NotFound
        );

        assert!(matches!(
            inspect_fixture_with_roots(
                &[("lib.rs", "manager::define_i18n_module!();")],
                "lib.rs",
                "define_i18n_module",
                &["manager"],
            ),
            InspectionOutcome::Found(_)
        ));
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
                SourceTarget::Macro("define_i18n_module", None)
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
            SourceTarget::Macro("define_i18n_module", None),
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
            SourceTarget::Macro("define_i18n_module", None),
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
            SourceTarget::Macro("define_i18n_module", None),
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
    fn explicit_path_submodules_resolve_beside_the_explicit_source() {
        let temp = tempfile::tempdir().expect("tempdir");
        let support = temp.path().join("support");
        fs::create_dir_all(&support).expect("create support directory");
        fs::write(
            temp.path().join("build.rs"),
            "#[path = \"support/helper_impl.rs\"] mod assets; fn main() { assets::run(); }\n",
        )
        .expect("write build target");
        fs::write(
            support.join("helper_impl.rs"),
            "mod nested; pub fn run() { nested::configure(); }\n",
        )
        .expect("write explicit module");
        let nested = support.join("nested.rs");
        fs::write(
            &nested,
            "pub fn configure() { es_fluent_build::track_i18n_assets(); }\n",
        )
        .expect("write nested module");

        let entry = temp.path().join("build.rs");
        let graph = reachable_source_graph(&entry, temp.path());
        assert!(
            graph.indeterminate_reasons.is_empty(),
            "valid explicit-path graph should be determinate: {:?}",
            graph.indeterminate_reasons
        );
        assert!(
            graph
                .paths
                .contains(&nested.canonicalize().expect("canonical nested module"))
        );
        assert!(matches!(
            inspect(&entry, temp.path(), SourceTarget::Call("track_i18n_assets")),
            InspectionOutcome::Found(_)
        ));
    }

    #[test]
    fn explicit_path_modules_may_resolve_beside_the_package() {
        let temp = tempfile::tempdir().expect("tempdir");
        let package = temp.path().join("app");
        let shared = temp.path().join("shared");
        fs::create_dir_all(&package).expect("create package directory");
        fs::create_dir_all(&shared).expect("create shared directory");
        let entry = package.join("build.rs");
        fs::write(
            &entry,
            "#[path = \"../shared/helper.rs\"] mod helper; fn main() { helper::run(); }\n",
        )
        .expect("write build target");
        let helper = shared.join("helper.rs");
        fs::write(
            &helper,
            "mod nested; pub fn run() { nested::configure(); }\n",
        )
        .expect("write shared helper");
        let nested = shared.join("nested.rs");
        fs::write(
            &nested,
            "pub fn configure() { es_fluent_build::track_i18n_assets(); }\n",
        )
        .expect("write nested shared helper");

        let graph = reachable_source_graph(&entry, &package);

        assert!(
            graph.indeterminate_reasons.is_empty(),
            "literal external module should be determinate: {:?}",
            graph.indeterminate_reasons
        );
        assert!(
            graph
                .paths
                .contains(&helper.canonicalize().expect("canonical shared helper"))
        );
        assert!(
            graph
                .paths
                .contains(&nested.canonicalize().expect("canonical nested helper"))
        );
        assert!(matches!(
            inspect(&entry, &package, SourceTarget::Call("track_i18n_assets")),
            InspectionOutcome::Found(_)
        ));
    }

    #[test]
    fn unresolved_external_explicit_path_records_nearest_watch_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let package = temp.path().join("app");
        let shared = temp.path().join("shared");
        fs::create_dir_all(&package).expect("create package directory");
        fs::create_dir_all(&shared).expect("create shared directory");
        let entry = package.join("build.rs");
        fs::write(
            &entry,
            "#[path = \"../shared/missing.rs\"] mod helper; fn main() {}\n",
        )
        .expect("write build target");

        let graph = reachable_source_graph(&entry, &package);

        assert!(!graph.indeterminate_reasons.is_empty());
        assert_eq!(graph.watch_dirs, vec![shared]);
    }

    #[cfg(unix)]
    #[test]
    fn explicit_path_symlinks_preserve_lexical_and_canonical_sources() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("tempdir");
        let package = temp.path().join("app");
        let shared = temp.path().join("shared");
        fs::create_dir_all(&package).expect("create package directory");
        fs::create_dir_all(&shared).expect("create shared directory");
        let entry = package.join("build.rs");
        fs::write(
            &entry,
            "#[path = \"helper.rs\"] mod helper; fn main() { helper::run(); }\n",
        )
        .expect("write build target");
        let target = shared.join("helper.rs");
        fs::write(
            &target,
            "include!(\"nested.rs\"); pub fn run() { configure(); }\n",
        )
        .expect("write helper target");
        let lexical_include = package.join("nested.rs");
        fs::write(
            &lexical_include,
            "fn configure() { es_fluent_build::track_i18n_assets(); }\n",
        )
        .expect("write lexical include");
        let canonical_sibling = shared.join("nested.rs");
        fs::write(&canonical_sibling, "fn configure() {}\n")
            .expect("write canonical target sibling");
        let lexical = package.join("helper.rs");
        symlink(&target, &lexical).expect("link helper");

        let graph = reachable_source_graph(&entry, &package);

        assert!(
            graph.indeterminate_reasons.is_empty(),
            "symlinked explicit module graph should be determinate: {:?}",
            graph.indeterminate_reasons
        );
        assert!(graph.lexical_paths.contains(&lexical));
        assert!(graph.lexical_paths.contains(&lexical_include));
        assert!(
            graph
                .paths
                .contains(&target.canonicalize().expect("canonical helper target"))
        );
        assert!(
            graph.paths.contains(
                &lexical_include
                    .canonicalize()
                    .expect("canonical lexical include")
            )
        );
        assert!(
            !graph.paths.contains(
                &canonical_sibling
                    .canonicalize()
                    .expect("canonical target sibling")
            )
        );
        assert!(matches!(
            inspect(&entry, &package, SourceTarget::Call("track_i18n_assets")),
            InspectionOutcome::Found(_)
        ));

        fs::remove_file(&lexical).expect("remove helper link");
        let missing_graph = reachable_source_graph(&entry, &package);
        assert!(missing_graph.lexical_paths.contains(&lexical));
        assert!(!missing_graph.indeterminate_reasons.is_empty());
        assert_eq!(missing_graph.watch_dirs, vec![package]);
    }

    #[test]
    fn included_submodules_resolve_from_the_include_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let support = temp.path().join("support");
        fs::create_dir_all(&support).expect("create support directory");
        fs::write(
            temp.path().join("build.rs"),
            "include!(\"support/config.rs\"); fn main() { configure(); }\n",
        )
        .expect("write build target");
        fs::write(
            support.join("config.rs"),
            "mod nested; fn configure() { nested::run(); }\n",
        )
        .expect("write included source");
        let nested = support.join("nested.rs");
        fs::write(
            &nested,
            "pub fn run() { es_fluent_build::track_i18n_assets(); }\n",
        )
        .expect("write nested module");

        let entry = temp.path().join("build.rs");
        let graph = reachable_source_graph(&entry, temp.path());
        assert!(
            graph.indeterminate_reasons.is_empty(),
            "valid include graph should be determinate: {:?}",
            graph.indeterminate_reasons
        );
        assert!(
            graph
                .paths
                .contains(&nested.canonicalize().expect("canonical nested module"))
        );
        assert!(matches!(
            inspect(&entry, temp.path(), SourceTarget::Call("track_i18n_assets")),
            InspectionOutcome::Found(_)
        ));
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
                SourceTarget::Macro("define_i18n_module", None)
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
    fn block_local_wrapper_shadowing_does_not_make_outer_helper_reachable() {
        let outcome = inspect_fixture(
            &[(
                "build.rs",
                "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { fn setup() {} setup(); }",
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets"),
        );

        assert!(matches!(outcome, InspectionOutcome::Indeterminate(_)));
    }

    #[test]
    fn local_value_bindings_do_not_make_outer_helpers_reachable() {
        for source in [
            "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { let setup = || {}; setup(); }",
            "fn setup() { es_fluent_build::track_i18n_assets(); } fn run(setup: impl Fn()) { setup(); } fn main() { run(|| {}); }",
            "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { let (setup, _) = (|| {}, 0); setup(); }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets"),
                ),
                InspectionOutcome::Indeterminate(_)
            ));
        }
    }

    #[test]
    fn control_flow_pattern_bindings_shadow_imported_helpers_lexically() {
        let imported_helper = "mod helper { pub fn setup() { es_fluent_build::track_i18n_assets(); } } use helper::setup;";
        for body in [
            "fn main() { for setup in [|| {}] { setup(); } }",
            "fn main() { match Some(|| {}) { Some(setup) if { setup(); true } => setup(), _ => {} } }",
            "fn main() { if let Some(setup) = Some(|| {}) { setup(); } }",
            "fn main() { while let Some(setup) = Some(|| {}) { setup(); break; } }",
            "fn main() { if let Some(setup) = Some(|| {}) && { setup(); true } { setup(); } }",
            "fn main() { while let Some(setup) = Some(|| {}) && { setup(); true } { setup(); break; } }",
        ] {
            let source = format!("{imported_helper} {body}");
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", &source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets"),
                ),
                InspectionOutcome::Indeterminate(reason)
                    if reason.contains("could not be proven reachable")
            ));
        }

        for body in [
            "fn main() { for setup in [setup()] { let _ = setup; } }",
            "fn main() { match setup() { setup => { let _ = setup; } } }",
            "fn main() { if let Some(setup) = setup() { let _ = setup; } }",
        ] {
            let source = format!("{imported_helper} {body}");
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", &source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets"),
                ),
                InspectionOutcome::Found(_)
            ));
        }

        let else_body =
            "fn main() { if let Some(setup) = Some(|| {}) { setup(); } else { setup(); } }";
        let source = format!("{imported_helper} {else_body}");
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", &source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets"),
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("control flow")
        ));
    }

    #[test]
    fn unsupported_block_imports_do_not_make_outer_helpers_reachable() {
        for import in [
            "use ::external_crate::setup;",
            "#[cfg(feature = \"external\")] use external_crate::setup;",
            "use external_crate::*;",
        ] {
            let source = format!(
                "fn setup() {{ es_fluent_build::track_i18n_assets(); }} fn main() {{ {import} setup(); }}"
            );
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", &source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets"),
                ),
                InspectionOutcome::Indeterminate(_)
            ));
        }
    }

    #[test]
    fn block_local_callable_items_do_not_make_outer_helpers_reachable() {
        for local_item in [
            "const setup: fn() = noop;",
            "static setup: fn() = noop;",
            "struct setup();",
        ] {
            let source = format!(
                "fn setup() {{ es_fluent_build::track_i18n_assets(); }} fn noop() {{}} fn main() {{ {local_item} setup(); }}"
            );
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", &source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets"),
                ),
                InspectionOutcome::Indeterminate(_)
            ));
        }
    }

    #[test]
    fn branch_guarded_build_helper_calls_are_indeterminate() {
        for source in [
            "fn main() { if false { es_fluent_build::track_i18n_assets(); } }",
            "fn main() { match false { true => es_fluent_build::track_i18n_assets(), false => {} } }",
            "fn main() { while false { es_fluent_build::track_i18n_assets(); } }",
            "fn main() { false && { es_fluent_build::track_i18n_assets(); true }; }",
            "fn main() { let _future = async { es_fluent_build::track_i18n_assets(); }; }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Indeterminate(reason)
                    if reason.contains("under control flow that could not be proven to execute")
            ));
        }
    }

    #[test]
    fn conditionally_reached_build_helper_functions_are_indeterminate() {
        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { if false { setup(); } }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("under control flow that could not be proven to execute")
        ));

        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { if false { setup(); } setup(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Found(_)
        ));
    }

    #[test]
    fn build_helper_calls_after_conditional_exits_are_indeterminate() {
        for source in [
            "fn skip() -> bool { false } fn main() { if skip() { return; } es_fluent_build::track_i18n_assets(); }",
            "fn skip() -> bool { false } fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { if skip() { return; } setup(); }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Indeterminate(reason)
                    if reason.contains("under control flow that could not be proven to execute")
            ));
        }
    }

    #[test]
    fn build_helper_calls_after_diverging_calls_are_indeterminate() {
        for source in [
            "fn main() { std::process::exit(0); es_fluent_build::track_i18n_assets(); }",
            "fn stop() -> ! { loop {} } fn main() { stop(); es_fluent_build::track_i18n_assets(); }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Indeterminate(reason)
                    if reason.contains("under control flow that could not be proven to execute")
            ));
        }
    }

    #[test]
    fn block_local_function_shadowing_build_helper_import_is_indeterminate() {
        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "use es_fluent_build::track_i18n_assets; fn main() { fn track_i18n_assets() {} track_i18n_assets(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("could not be resolved to the expected es-fluent dependency")
        ));
    }

    #[test]
    fn local_binding_shadowing_build_helper_import_is_indeterminate() {
        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "use es_fluent_build::track_i18n_assets; fn main() { let track_i18n_assets = || {}; track_i18n_assets(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("could not be resolved to the expected es-fluent dependency")
        ));

        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "use es_fluent_build::track_i18n_assets; fn main() { let track_i18n_assets = { track_i18n_assets(); || {} }; }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Found(_)
        ));
    }

    #[test]
    fn build_helper_calls_after_return_are_not_found() {
        assert_eq!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "fn main() { return; es_fluent_build::track_i18n_assets(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::NotFound
        );
    }

    #[test]
    fn build_helper_calls_after_nested_return_are_not_found() {
        assert_eq!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "fn main() { { return; } es_fluent_build::track_i18n_assets(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::NotFound
        );
    }

    #[test]
    fn build_helper_calls_after_diverging_loops_do_not_pass() {
        assert_eq!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "fn main() { loop {} es_fluent_build::track_i18n_assets(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::NotFound
        );

        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { loop { continue; } setup(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("could not be proven reachable")
        ));
    }

    #[test]
    fn build_helper_calls_after_wrapped_diverging_loops_do_not_pass() {
        for source in [
            "fn main() { let _never = loop {}; es_fluent_build::track_i18n_assets(); }",
            "fn main() { let mut value = (); value = { loop {} }; es_fluent_build::track_i18n_assets(); }",
        ] {
            assert_eq!(
                inspect_fixture(
                    &[("build.rs", source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::NotFound
            );
        }
    }

    #[test]
    fn loops_in_deferred_or_item_bodies_do_not_hide_following_build_helpers() {
        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "fn main() { let _closure = || loop {}; let _future = async { loop {} }; fn stop() { loop {} } es_fluent_build::track_i18n_assets(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Found(_)
        ));
    }

    #[test]
    fn build_helper_calls_after_loops_with_breaks_are_found() {
        for source in [
            "fn main() { loop { break; } es_fluent_build::track_i18n_assets(); }",
            "fn main() { let _value = loop { break; }; es_fluent_build::track_i18n_assets(); }",
            "fn main() { let mut value = (); value = loop { break; }; es_fluent_build::track_i18n_assets(); }",
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
    fn build_helper_calls_after_conditionally_breaking_loops_are_indeterminate() {
        for source in [
            "fn main() { loop { if runtime_condition() { break; } } es_fluent_build::track_i18n_assets(); }",
            "fn main() { loop { if runtime_condition() { continue; } break; } es_fluent_build::track_i18n_assets(); }",
            "fn main() { let _value = loop { if runtime_condition() { break; } }; es_fluent_build::track_i18n_assets(); }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Indeterminate(reason)
                    if reason.contains("under control flow that could not be proven to execute")
            ));
        }
    }

    #[test]
    fn local_module_shadowing_build_dependency_is_indeterminate() {
        for source in [
            "mod es_fluent_build { pub fn track_i18n_assets() {} } fn main() { es_fluent_build::track_i18n_assets(); }",
            "mod local { pub mod es_fluent_build { pub fn track_i18n_assets() {} } } use local::es_fluent_build; fn main() { es_fluent_build::track_i18n_assets(); }",
            "mod local { pub fn track_i18n_assets() {} } use local as es_fluent_build; fn main() { es_fluent_build::track_i18n_assets(); }",
            "extern crate self as es_fluent_build; fn track_i18n_assets() {} fn main() { es_fluent_build::track_i18n_assets(); }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Indeterminate(reason)
                    if reason.contains("could not be resolved to the expected es-fluent dependency")
            ));
        }
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
                SourceTarget::Macro("define_i18n_module", None)
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
                SourceTarget::Macro("define_i18n_module", None)
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
                SourceTarget::Macro("define_i18n_module", None)
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
                SourceTarget::Macro("define_i18n_module", None)
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

    #[test]
    fn opaque_item_macro_expansions_are_indeterminate() {
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", "configure_i18n!(); fn main() {}")],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("opaque item macro expansion")
        ));
    }

    #[test]
    fn verified_calls_with_opaque_item_macro_expansions_are_indeterminate() {
        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    r#"macro_rules! define_local_helper {
    () => {
        mod es_fluent_build {
            pub fn track_i18n_assets() {}
        }
    };
}
define_local_helper!();
fn main() { es_fluent_build::track_i18n_assets(); }
"#
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("opaque item macro expansion")
        ));
    }

    #[test]
    fn opaque_statement_macro_expansions_are_indeterminate() {
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", "fn main() { configure_i18n!(); }")],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("opaque statement macro expansion")
        ));
    }

    #[test]
    fn build_helper_calls_after_opaque_macros_are_indeterminate() {
        for source in [
            "fn main() { panic!(\"stop\"); es_fluent_build::track_i18n_assets(); }",
            "fn main() { configure_i18n!(); es_fluent_build::track_i18n_assets(); }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Indeterminate(reason)
                    if reason.contains("under control flow that could not be proven to execute")
            ));
        }
    }

    #[test]
    fn opaque_helper_references_are_indeterminate() {
        for source in [
            "use es_fluent_build::track_i18n_assets; fn main() { let f: fn() = track_i18n_assets; f(); }",
            "fn main() { let f: fn() = es_fluent_build::track_i18n_assets; f(); }",
        ] {
            assert!(matches!(
                inspect_fixture(
                    &[("build.rs", source)],
                    "build.rs",
                    SourceTarget::Call("track_i18n_assets")
                ),
                InspectionOutcome::Indeterminate(reason)
                    if reason.contains("opaque reference to `track_i18n_assets`")
            ));
        }
    }

    #[test]
    fn opaque_expression_macro_expansions_are_indeterminate() {
        assert!(matches!(
            inspect_fixture(
                &[(
                    "build.rs",
                    "fn main() { let _configuration = configure_i18n!(); }"
                )],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("opaque expression macro expansion")
        ));
    }

    #[test]
    fn source_graph_marks_macro_wrapped_include_indeterminate_without_a_doctor_target() {
        let temp = tempfile::tempdir().expect("tempdir");
        let support = temp.path().join("support");
        fs::create_dir_all(&support).expect("create support directory");
        fs::write(
            temp.path().join("build.rs"),
            "macro_rules! load_config { () => { include!(\"support/config.rs\"); }; } load_config!(); fn main() {}\n",
        )
        .expect("write build target");
        fs::write(support.join("config.rs"), "pub fn configure() {}\n")
            .expect("write included source");

        let graph = reachable_source_graph(&temp.path().join("build.rs"), temp.path());

        assert!(
            graph
                .indeterminate_reasons
                .iter()
                .any(|reason| { reason.contains("macro wrapper") && reason.contains("include") })
        );
        assert!(
            graph
                .indeterminate_reasons
                .iter()
                .any(|reason| reason.contains("opaque item macro expansion"))
        );
    }
}

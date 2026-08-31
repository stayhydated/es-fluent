use super::*;

pub(super) fn analyze_reachability(
    sources: &[ParsedSource],
    entry_path: &Path,
) -> ReachabilityAnalysis {
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

pub(super) fn has_conditional_attr(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
}

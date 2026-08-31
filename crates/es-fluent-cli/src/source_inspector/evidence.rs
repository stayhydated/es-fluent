use super::*;

pub(super) struct IncludeSource {
    pub(super) path: Option<PathBuf>,
    pub(super) line: usize,
    pub(super) conditional: bool,
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

pub(super) struct EvidenceVisitor<'a> {
    target: Option<SourceTarget<'a>>,
    current_file: &'a Path,
    conditional_depth: usize,
    current_function: Option<FunctionLocation>,
    execution_uncertain_depth: usize,
    diverging_functions: HashSet<String>,
    pub(super) evidences: Vec<MatchedEvidence>,
    pub(super) includes: Vec<IncludeSource>,
    scopes: Vec<ImportResolution>,
    pub(super) indeterminate_reasons: Vec<String>,
}

impl<'a> EvidenceVisitor<'a> {
    pub(super) fn new(
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

pub(super) fn statement_unconditionally_terminates(statement: &syn::Stmt) -> bool {
    match statement {
        syn::Stmt::Local(local) => local
            .init
            .as_ref()
            .is_some_and(|init| expression_unconditionally_terminates(&init.expr)),
        syn::Stmt::Expr(expression, _) => expression_unconditionally_terminates(expression),
        syn::Stmt::Item(_) | syn::Stmt::Macro(_) => false,
    }
}

pub(super) fn statement_may_skip_following(
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

pub(super) fn diverging_function_names(file: &syn::File) -> HashSet<String> {
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

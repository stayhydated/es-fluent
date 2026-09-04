use super::*;

pub(super) fn inspect_source_graph(
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
    graph.watch_dirs = graph
        .watch_dirs
        .into_iter()
        .map(|path| crate::utils::paths::normalize_windows_verbatim_path(&path))
        .collect();
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

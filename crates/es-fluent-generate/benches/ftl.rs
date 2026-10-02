use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use es_fluent_generate::{formatting::sort_ftl_resource, ftl::extract_variables_from_message};
use fluent_syntax::{ast, parser};
use std::fmt::Write as _;
use std::hint::black_box;

fn sorting_resource(entries: usize, groups: usize, layout: &str) -> ast::Resource<String> {
    let mut source = String::new();
    for group in (0..groups).rev() {
        writeln!(source, "## Group{group}\n").unwrap();
        for index in (0..entries).rev() {
            let owner = index % groups;
            if (layout == "misgrouped" && group == 0) || (layout != "misgrouped" && owner == group)
            {
                let prefix = if index % 4 == 0 { "-" } else { "" };
                let label = if index % 10 == 0 { "_label" } else { "" };
                writeln!(source, "{prefix}group{owner}_value{index}{label} = A representative translation for item {index}").unwrap();
                writeln!(source, "    .tooltip = Detail for item {index}\n").unwrap();
            }
        }
    }
    let resource = parser::parse(source).expect("sorting fixture must parse");
    if layout == "sorted" {
        parser::parse(sort_ftl_resource(&resource)).expect("formatted fixture must parse")
    } else {
        resource
    }
}

fn sorting(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ftl_sort");
    for (entries, sections) in [(16, 1), (128, 8), (1024, 64)] {
        group.throughput(Throughput::Elements(entries as u64));
        for layout in ["sorted", "reversed", "misgrouped"] {
            let resource = sorting_resource(entries, sections, layout);
            group.bench_with_input(
                BenchmarkId::new(layout, entries),
                &resource,
                |bencher, resource| {
                    bencher.iter(|| sort_ftl_resource(black_box(resource)));
                },
            );
        }
    }
    group.finish();
}

fn traversal_message(references: usize, depth: usize) -> ast::Message<String> {
    let mut source = "message = Text".to_owned();
    for index in 0..references {
        let mut expression = format!("$var{index}");
        for _ in 0..depth {
            expression = format!("{{ {expression} }}");
        }
        write!(source, " {{ {expression} }}").unwrap();
    }
    source.push_str("\n    .detail = { FUNC($var0, style: \"short\") }\n    .choice = { $selector ->\n        [one] One { $var0 }\n       *[other] Other { $selector }\n    }\n");
    let mut resource = parser::parse(source).expect("traversal fixture must parse");
    let ast::Entry::Message(message) = resource.body.remove(0) else {
        panic!("traversal fixture must contain its message");
    };
    message
}

fn traversal(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ftl_variable_traversal");
    for references in [1, 16, 64] {
        // The fixed attributes contain four additional variable references.
        group.throughput(Throughput::Elements((references + 4) as u64));
        for depth in [0, 2, 4] {
            let message = traversal_message(references, depth);
            group.bench_with_input(
                BenchmarkId::new(format!("depth_{depth}"), references),
                &message,
                |bencher, message| {
                    bencher.iter(|| extract_variables_from_message(black_box(message)));
                },
            );
        }
    }
    group.finish();
}

criterion_group!(benches, sorting, traversal);
criterion_main!(benches);

//! Benchmarks for expression evaluation and integration hot paths.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use numerical_approximater::app::{Method, MAX_GRAPH_POINTS};
use numerical_approximater::expr::OdeFunction;
use numerical_approximater::solver::{integrate, subsample_plot};

/// Single `eval` call — dominated by parse-tree reuse vs re-parse per call.
fn bench_eval(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval");

    let simple = OdeFunction::parse("x + y").unwrap();
    group.bench_function("x_plus_y", |b| {
        let mut x = 0.0;
        b.iter(|| {
            x += 0.001;
            black_box(simple.eval(black_box(x), black_box(x * 0.5)).unwrap())
        });
    });

    let heavy = OdeFunction::parse("sin(x)*y + e^x").unwrap();
    group.bench_function("sin_y_plus_exp_x", |b| {
        let mut x = 0.0;
        b.iter(|| {
            x += 0.001;
            black_box(heavy.eval(black_box(x), black_box(x * 0.5)).unwrap())
        });
    });

    group.finish();
}

/// Full fixed-step integration — many `eval` calls per run.
fn bench_integrate(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrate");

    let simple = OdeFunction::parse("x + y").unwrap();
    group.bench_function("euler_x_plus_y_0_10_h001", |b| {
        b.iter(|| {
            black_box(
                integrate(
                    &simple,
                    Method::Euler,
                    0.0,
                    1.0,
                    10.0,
                    0.01,
                )
                .unwrap(),
            )
        });
    });

    let heavy = OdeFunction::parse("sin(x)*y + e^x").unwrap();
    for (name, method) in [
        ("euler", Method::Euler),
        ("improved_euler", Method::ImprovedEuler),
        ("rk4", Method::RungeKutta),
    ] {
        group.bench_function(
            &format!("{name}_sin_y_plus_exp_x_0_10_h001"),
            |b| {
                b.iter(|| {
                    black_box(
                        integrate(&heavy, method, 0.0, 1.0, 10.0, 0.01).unwrap(),
                    )
                });
            },
        );
    }

    group.finish();
}

/// Typical UI workload: three methods, subsample for plotting.
fn bench_recompute_workload(c: &mut Criterion) {
    c.bench_function("recompute_three_methods_rk4_family", |b| {
        b.iter(|| {
            let f = OdeFunction::parse("sin(x)*y + e^x").unwrap();
            let x0 = 0.0;
            let y0 = 1.0;
            let x_end = 10.0;
            let h = 0.01;
            let mut curves = Vec::new();
            for method in [
                Method::Euler,
                Method::ImprovedEuler,
                Method::RungeKutta,
            ] {
                let points = integrate(&f, method, x0, y0, x_end, h).unwrap();
                let plot_xy = subsample_plot(&points, MAX_GRAPH_POINTS);
                curves.push((points.len(), plot_xy.len()));
            }
            black_box(curves)
        });
    });
}

/// Plot subsampling after a long integration (Vec preallocation).
fn bench_subsample_plot(c: &mut Criterion) {
    let f = OdeFunction::parse("x + y").unwrap();
    let points = integrate(&f, Method::RungeKutta, 0.0, 0.0, 10.0, 0.001).unwrap();

    c.bench_function("subsample_10k_to_800", |b| {
        b.iter(|| black_box(subsample_plot(&points, MAX_GRAPH_POINTS)));
    });
}

criterion_group!(
    benches,
    bench_eval,
    bench_integrate,
    bench_recompute_workload,
    bench_subsample_plot
);
criterion_main!(benches);

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use tacky_borders::{check_env, env, resolve_env_vars};

fn bench_functions(c: &mut Criterion) {
    // Test cases with more descriptive input names, including default values for variables
    let cases = vec![
        ("simple_env_variable", "%USERPROFILE%/$APPDATA/.config"), // A simple case with basic environment variables
        (
            "complex_env_variables",
            "%USERPROFILE%/Documents/%APPDATA%/.config", // Multiple variables in the path
        ),
        (
            "nested_env_variables",
            "${USERPROFILE}/AppData/%APPDATA%/config", // Nested variables with both ${} and %%
        ),
        (
            "long_env_path_with_variables",
            "%USERPROFILE%/Some/Long/Path/To/Check/$APPDATA/.config/long/another/path", // A long path with multiple variables
        ),
        (
            "default_value_in_env_variable_percent",
            "%A=B%/Config/settings", // Using default value with %A=B%
        ),
        (
            "default_value_in_env_variable_braces",
            "${A:B}/Config/settings", // Using default value with ${A:B}
        ),
    ];

    let mut group = c.benchmark_group("env_resolution");

    for (name, input) in cases {
        // Benchmark for `env` function with a descriptive label
        group.bench_function(format!("env_function/{}", name), |b| {
            b.iter(|| black_box(env(black_box(input)).unwrap()))
        });

        // Benchmark for `check_env` function with a descriptive label
        group.bench_function(format!("check_env_function/{}", name), |b| {
            b.iter(|| black_box(check_env(black_box(input)).unwrap()))
        });

        // Benchmark for `resolve_env_vars` function with a descriptive label
        group.bench_function(format!("resolve_env_vars_function/{}", name), |b| {
            b.iter(|| black_box(resolve_env_vars(black_box(input)).unwrap()))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_functions);
criterion_main!(benches);

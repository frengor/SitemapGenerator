use criterion::{black_box, Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;
use url::Url;

use sitemap_generator::{analyze, Options, Validator};

fn benchmark(c: &mut Criterion) {
    let options = Options::builder().set_remove_query_and_fragment(true).build();
    let domain = std::iter::once(Url::parse("https://frengor.com").unwrap());
    let validator = Validator::new(domain);
    let ulaapi = std::iter::once(Url::parse("https://frengor.com/UltimateAdvancementAPI/").unwrap());
    let javadocs = std::iter::once(Url::parse("https://frengor.com/javadocs/").unwrap());

    c.bench_function("sitemap_small", |b| b.to_async(start_runtime()).iter(|| {
        analyze(black_box(ulaapi.clone()), black_box(validator.clone()), black_box(options))
    }));
    c.bench_function("sitemap_medium", |b| b.to_async(start_runtime()).iter(|| {
        analyze(black_box(javadocs.clone()), black_box(validator.clone()), black_box(options))
    }));
}

#[inline]
fn start_runtime() -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .thread_name("SitemapGenerator-bench")
    .build()
    .expect("Failed building the Runtime")
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = benchmark
}
criterion_main!(benches);

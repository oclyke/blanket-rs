# blanket-rs
static generator

to get started try the example: `cargo run --example basic`

then go ahead and start using the library in your own project =D

`cargo add blanket-rs`

```rust
fn main() {
    fn run() -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = blanket_rs::builder::Builder::new();
        builder.require(blanket_rs::resource::CopyFile::new("source/index.html", "dest/index.html"))?;
        builder.generate()?;
        Ok(())
    }
    run().expect("expected to exit successfully");
}
```

# why blanket-rs
great question. there are a lot of options for static website generation in
Rust - see [alternatives](#alternatives) - but for many use cases they are
overkill. blanket is all about simplicity.

**some simple pleasures**
* you are in control
* you add blanket to your project, not the other way around

**you should use blanket**
* to declaratively generate a static website

**you should not use blanket**
* if you need to compile or bundle javascript (check out [vite](https://github.com/vitejs/vite)!)
* when *performance* is as imporant as correctness (check out [bazel](https://github.com/bazelbuild/bazel)!)

# pairings
some flavors that compliment blanket-rs
* JSX style `<Component />` syntax tools like [render](https://github.com/render-rs/render.rs)
* markdown parsers, like [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) or [markdown-rs](https://github.com/wooorm/markdown-rs)

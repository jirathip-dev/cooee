//! Strips the types from each `web/ts/*.ts` into `$OUT_DIR/<name>.js` with oxc (#49): one ES file per entry, no
//! bundler and no Node; `lib.rs` embeds them. A file that doesn't parse or strip fails the build at its file and line.

use std::{env, fs, path::Path, process::exit};

use oxc::{
    allocator::Allocator,
    codegen::{Codegen, CodegenOptions, CommentOptions},
    diagnostics::OxcDiagnostic,
    parser::Parser,
    semantic::SemanticBuilder,
    span::SourceType,
    transformer::{TransformOptions, Transformer},
};

fn main() {
    println!("cargo::rerun-if-changed=ts");
    let out = env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    let mut entries: Vec<_> = fs::read_dir("ts")
        .expect("web/ts")
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "ts"))
        .filter(|p| !p.to_string_lossy().ends_with(".d.ts"))
        .collect();
    entries.sort();
    let mut failed = false;
    for path in &entries {
        let source = fs::read_to_string(path).expect("a readable .ts file");
        match strip(path, &source) {
            Ok(js) => {
                let name = path.with_extension("js");
                let name = name.file_name().expect("a file name");
                fs::write(Path::new(&out).join(name), js).expect("OUT_DIR is writable");
            }
            Err(errors) => {
                failed = true;
                for e in errors {
                    let at = e.labels.first().map_or(0, |l| l.offset() as usize);
                    let line = source[..at.min(source.len())].matches('\n').count() + 1;
                    eprintln!("web/{}:{line}: {}", path.display(), e.message);
                }
            }
        }
    }
    if failed {
        exit(1);
    }
}

fn strip(path: &Path, source: &str) -> Result<String, Vec<OxcDiagnostic>> {
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, source, SourceType::ts()).parse();
    if !parsed.diagnostics.is_empty() {
        return Err(parsed.diagnostics.into_vec());
    }
    let mut program = parsed.program;
    let semantic = SemanticBuilder::new().build(&program);
    if !semantic.diagnostics.is_empty() {
        return Err(semantic.diagnostics.into_vec());
    }
    let scoping = semantic.semantic.into_scoping();
    let done = Transformer::new(&alloc, path, &TransformOptions::default())
        .build_with_scoping(scoping, &mut program);
    if done.diagnostics.has_errors() {
        return Err(done.diagnostics.into_vec());
    }
    let options = CodegenOptions {
        comments: CommentOptions::disabled(),
        ..CodegenOptions::default()
    };
    Ok(Codegen::new().with_options(options).build(&program).code)
}

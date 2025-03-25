use relative_path::PathExt;
use std::env;
use std::path::Path;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir().unwrap();
    let dir = "--dir".to_string();
    let mut args = vec!["run".to_string()];
    let env_vars = ["OUT_DIR", "CARGO_MANIFEST_DIR"];
    let mut cargo_tomls = Vec::new();

    // Starting from the manifest dir down to the root of the filesystem
    //
    // If cargo.toml exists append it to the list
    // FIXME: this should be improved, it would be wise to
    // read the Cargo.toml and look for a `[workspace]` key
    // then use the first `Cargo.toml` with one of those.
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut cur_dir = Some(Path::new(&manifest_dir));
        while let Some(path) = cur_dir {
            let cargo_toml_path = path.with_file_name("Cargo.toml");
            if cargo_toml_path.exists() {
                cargo_tomls.push(
                    cargo_toml_path
                        .relative_to(cwd.clone())
                        .unwrap()
                        .to_path("."),
                );
            }
            cur_dir = path.parent();
        }
    }

    // Add the paths in `ENV_VARS` --env converting them relative to the current directory
    // Add the --dir preopens too.
    for var in env_vars {
        if let Ok(env_dir) = std::env::var(var) {
            let rel = if Path::new(&env_dir) == cwd.clone() {
                ".".to_string()
            } else {
                Path::new(&env_dir).relative_to(cwd.clone())?.to_string()
            };
            args.extend(["--env".to_string(), format!("{var}={}", &rel.clone())]);
            args.extend([dir.clone(), rel.to_string()]);
        };
    }

    // For the last `Cargo.toml` in the list
    // add `--dir` for each directory from the current directorty `--dir .`,
    // `--dir ..` `--dir ../..`, and so on up to the directory of the `Cargo.toml`.
    for ancestor in cargo_tomls.last().unwrap().parent().unwrap().ancestors() {
        let path_arg = ancestor.display().to_string();
        if !path_arg.is_empty() {
            args.extend([dir.clone(), path_arg]);
        }
    }

    // Now add `--dir .`, even if this has been added previously,
    // the last one will set the current working directory
    args.extend([dir.clone(), ".".to_string()]);
    // cargo test will run the "runner" with args, skip argv[0]
    args.extend(std::env::args().skip(1));
    let mut child = Command::new("wasmtime")
        .args(args)
        .spawn()
        .expect("failed to execute process");
    child.wait().expect("command wasn't running");
    Ok(())
}

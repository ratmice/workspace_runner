use relative_path::PathExt;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(serde::Deserialize)]
struct CargoMetadata {
    workspace_root: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir().unwrap();
    let dir: OsString = "--dir".into();
    let mut args: Vec<OsString> = vec!["run".into()];
    let env_vars = ["OUT_DIR", "CARGO_MANIFEST_DIR"];
    let output = Command::new("cargo").args(["metadata"]).output()?;
    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)?;

    let to_workspace = metadata.workspace_root.relative_to(&cwd)?.to_path("");

    // wasmtime is very finicky in what it accepts,
    // e.g. --dir "./.." fails while ".." works.
    //
    // --dir "../.." fails
    // while --dir "../.." --dir ".." works.
    //
    //So we
    // --dir ".", --dir ".." --dir "../.."
    // all the way up to the workspace_root.
    //
    // this is also why we don't just add "--dir workspace_root"
    for path in to_workspace.ancestors() {
        let path_str = OsString::from(path);
        if !path.as_os_str().is_empty() {
            args.extend([dir.clone(), path_str])
        }
    }

    // Add the paths in `ENV_VARS` --env converting them relative to the current directory
    // Add the --dir preopens too.
    for var in env_vars {
        if let Ok(env_dir) = std::env::var(var) {
            let rel: OsString = if Path::new(&env_dir) == cwd.clone() {
                ".".into()
            } else {
                Path::new(&env_dir)
                    .relative_to(cwd.clone())?
                    .to_path("")
                    .into()
            };

            let mut rel_str = OsString::new();
            rel_str.push(var);
            rel_str.push("=");
            rel_str.push(rel.clone());
            args.extend(["--env".into(), rel_str]);
            args.extend([dir.clone(), rel]);
        };
    }

    // Now add `--dir .`, even if this has been added previously,
    // the last one will set the current working directory
    args.extend([dir.clone(), ".".into()]);
    // cargo test will run the "runner" with args, skip argv[0]
    args.extend(std::env::args_os().skip(1));
    let mut child = Command::new("wasmtime")
        .args(args)
        .spawn()
        .expect("failed to execute process");
    child.wait().expect("command wasn't running");
    Ok(())
}

use relative_path::PathExt;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

#[derive(serde::Deserialize)]
struct CargoMetadata {
    workspace_root: PathBuf,
}

enum Target {
    Wasm32WasiP2,
}

// I couldn't find an arg parser which I liked enough
// and was light weight enough, partially because the
// few I tried ended up pulling additional versions of
// serde and syn.
//
// I'm already disappointed enough that this crate
// pulls in one version of those for cargo metadata.
//
// The argument parsing needs are pretty trivial
// But it would be better to use a tested crate,
// assuming it handles ArgsOs appropriately.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // handle args and strip all the args we know.
    let mut args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let dash_dash = args.iter().position(|n| n == "--");
    let target: Target = if let Some(dash_dash) = dash_dash {
        args.remove(dash_dash);
        let prog_args = args.drain(0..dash_dash).collect::<Vec<OsString>>();
        match prog_args {
            x if x == ["--target", "wasm32-wasip2"] => Target::Wasm32WasiP2,
            _ => {
                if prog_args.contains(&"--target".into()) {
                    eprintln!(
                        "unrecognized target: \"{}\"",
                        prog_args
                            .iter()
                            .filter(|x| x != &"--target")
                            .map(|x| x.to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                } else {
                    eprintln!("missing argument: --target t");
                }
                exit(-1);
            }
        }
    } else {
        eprintln!("-- argument is required.");
        exit(-1);
    };

    match target {
        Target::Wasm32WasiP2 => run_wasmtime_wasip2(args),
    }
}

fn run_wasmtime_wasip2(os_args: Vec<OsString>) -> Result<(), Box<dyn std::error::Error>> {
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
    args.extend(os_args);
    let mut child = Command::new("wasmtime")
        .args(args)
        .spawn()
        .expect("failed to execute process");
    child.wait().expect("command wasn't running");
    Ok(())
}

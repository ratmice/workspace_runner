`workspace_runner` runs wasmtime with the appropriate 
`--dir` flags set so that wasmtime can access your entire
workspace via the filesystem.

It also adds `--env` flags for environment variables set by
cargo that specify directories.

You can use this in a `.cargo/config.toml` such as the following

```
[target.wasm32-wasip2]
runner = "workspace_runner --target wasm32-wasip2 --"
```

Currently the only supported target is `wasm32-wasip2`.


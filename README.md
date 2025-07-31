CI tool
=======

`ci` does tasks for Servo’s CI builds.

`ci` is a simple Rust CLI that uses [clap](https://docs.rs/clap/4.5.42/clap/), [eyre](https://docs.rs/eyre/0.6.12/eyre/), [tracing](https://docs.rs/tracing/0.1.41/tracing/), plus [cmd_lib](https://docs.rs/cmd_lib/1.9.6/cmd_lib/) for shelling out to other commands.
It uses [cargo-dist](https://axodotdev.github.io/cargo-dist/) to automatically publish prebuilt binaries to [the Releases page](https://github.com/delan/servo-ci/releases).

## How to use it in a GitHub Actions job

```yaml
jobs:
  my-job:
    steps:
      - uses: delan/servo-ci@main
      - run: ci hello
      # Output: hello world
```

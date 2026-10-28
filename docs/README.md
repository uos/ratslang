# Documentation

Install [uv](https://docs.astral.sh/uv/) and use Python 3.11 or newer. From the
Ratslang repository root:

```sh
mask docs
mask docs-serve
```

`docs/requirements.txt`, the mask tasks, and the CI workflow pin the same Zensical
release. Update them together.

## Deployment

The Forgejo workflow builds and checks documentation on pull requests and pushes.
On `main` pushes, release tags, or a manual run on `main`, it deploys `docs/site/`
to `https://ratslang.steado.tech/docs/`. The workflow uses the runner's existing Rust, Python, and uv.

The published Rust API stays on docs.rs. This site explains the language,
integrations, and tools. Keep the root README brief and link readers here.

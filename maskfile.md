# Ratslang development tasks

## docs-check

> Validate documentation links and compile the language and Rust examples.

```sh
python3 scripts/check_docs.py
```

## docs

> Check examples and build the Zensical site into docs/site.

```sh
python3 scripts/check_docs.py
cd docs && uvx --from zensical==0.0.71 --with ./pygments-ratslang zensical build --clean --strict
```

## docs-serve

> Check examples and preview documentation with live reload.

```sh
python3 scripts/check_docs.py
cd docs && uvx --from zensical==0.0.71 --with ./pygments-ratslang zensical serve --dev-addr 127.0.0.1:8000
```

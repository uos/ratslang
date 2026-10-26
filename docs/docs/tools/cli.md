# Command-line tools

The packages contain the CLI and the [language server](../editors/index.md).

=== "Ubuntu"

    Add the steado PPA once, then install the package:

    ```sh
    curl -fsSL "https://ppa.steado.tech/ubuntu/key.gpg" | gpg --dearmor \
      | sudo tee /usr/share/keyrings/steado-archive-keyring.gpg >/dev/null
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/steado-archive-keyring.gpg] https://ppa.steado.tech/ubuntu $(. /etc/os-release && echo $UBUNTU_CODENAME) main" \
      | sudo tee /etc/apt/sources.list.d/steado.list
    sudo apt update
    sudo apt install ratslang
    ```

=== "Arch Linux"

    ```sh
    yay -S ratslang-bin
    ```

    `ratslang` builds from source instead.

=== "macOS"

    ```sh
    brew install uos/tools/ratslang
    ```

=== "Python"

    ```sh
    uv tool install ratslang-cli
    ```

    `pipx install ratslang-cli` and `pip install ratslang-cli` work too.

=== "Cargo"

    [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) downloads the
    release binary instead of compiling:

    ```sh
    cargo binstall ratslang-cli
    ```

    To compile it:

    ```sh
    cargo install ratslang-cli --locked
    ```

The examples below use a rat's defaults and its Friday overrides.

```ratslang title="default.rl"
# How far the whiskers reach.
# @type length
# @min 1cm
# @max 10cm
# @recommended
rat.whisker.length = 4cm

# @enum CURIOUS, SNEAKY, HANGRY
rat.mood = CURIOUS

# Old name of rat.nap_after.
# @type time
# @renamed_to rat.nap_after
rat.siesta = 20min

rat.nap_after = 20min
```

```ratslang title="friday.rl"
# @schema ./default.rl
rat.whisker.length=5cm
rat.mood  = CURIOUS
rat.siesta = 45min
```

## Check

```text
$ ratslang check friday.rl
warning: rat.siesta: This setting was renamed; use rat.nap_after
friday.rl: valid
```

Add `--strict` to fail on warnings in CI. Errors always fail:

```text
$ ratslang check friday.rl --set rat.whisker.length=15cm
rat.whisker.length: value 15cm must be <= 10cm

$ ratslang check friday.rl --set rat.mood=GRUMPY
rat.mood: expected one of [CURIOUS, SNEAKY, HANGRY], got "GRUMPY"
```

The schema comes from the `# @schema` line. Pass `--schema default.rl` for a
file without one. The CLI reads local schemas only. The editor also fetches
HTTPS ones.

## Explain

Where a value comes from, layer by layer:

```text
$ ratslang explain friday.rl rat.whisker.length --set rat.whisker.length=6cm
rat.whisker.length = 6cm
Winner [CLI]: <command line>:1:1
Assignment history:
  [default] /home/you/maze/default.rl:6:1: rat.whisker.length = 4cm -> 4cm
    included from <layers>:1
  [file] /home/you/maze/friday.rl:2:1: rat.whisker.length = 5cm -> 5cm
    included from <layers>:1
  [CLI] <command line>:1:1: rat.whisker.length = 6cm -> 6cm
```

## Format

```text
$ ratslang fmt friday.rl
# @schema ./default.rl
rat.whisker.length = 5cm
rat.mood = CURIOUS
rat.siesta = 45min
```

## Generate a starter file

```text
$ ratslang generate default.rl --schema-url https://example.org/default.rl
# @schema https://example.org/default.rl
# How far the whiskers reach.
rat.whisker.length = 4cm
```

`--full` includes every setting, not only the required and recommended ones.

## Minimize

Comments out overrides that equal the default:

```text
$ ratslang minimize friday.rl
# @schema ./default.rl
rat.whisker.length=5cm
# rat.mood  = CURIOUS
rat.siesta = 45min
```

## Migrate

Moves settings marked `@renamed_to`:

```text
$ ratslang migrate friday.rl
# @schema ./default.rl
rat.whisker.length=5cm
rat.mood  = CURIOUS
# rat.siesta = 45min
rat.nap_after = 45min
```

`fmt`, `minimize` and `migrate` print the result. Add `--write` to change the
file. Nothing is written if the values would change.

---
hide:
  - toc
---

<div class="sk-intro">
  <div class="sk-intro-copy">

<div class="sk-wordmark"><img src="images/ratslang.svg" alt="" width="72" height="72"><h1>Ratslang</h1></div>

<p class="sk-tagline">Configuration language for<br>building physical systems in Rust.</p>

<div class="sk-actions">
  <a class="md-button md-button--primary" href="getting-started/">Get started</a>
  <a class="md-button" href="language/values/">Explore the language</a>
</div>

  </div>
  <div class="sk-intro-visual" markdown>

```ratslang title="rat.rl"
# A robot rat for maze trials.
rat {
  # @type length
  # @min 1cm
  # @max 10cm
  whisker.length = 4cm

  # @type velocity
  # @max 2m/s
  scurry_speed = 1.2m/s

  # @enum CURIOUS, SNEAKY, HANGRY
  mood = CURIOUS

  nap_after = 20min
}
```

  </div>
</div>

<div class="grid cards" markdown>

- **The unit is part of the value**

    `36km/h` and `10m/s` are the same speed on a type-level. A bound of `@max 2m` rejects
    `250cm`, whichever unit either side is written in.

    [Units and dimensions →](language/units.md)

- **The default file is the schema**

    `@type`, `@min` and `@enum` comments in your default check user files and
    drive completion and hover text in the editor.

    [Schemas and constraints →](schemas.md)

- **Deserialize into Rust structs**

    Serde reads values into `uom` quantities, so a length stays a length without losing conversion accuracy.

    [Serde integration →](rust/serde.md)

- **LSP for your editor**

    Completion, hover docs, diagnostics and quick fixes from the same schema
    rules the application checks. Set up for Helix, Neovim and VS Code.

    [Editor setup →](editors/index.md)

</div>

Ratslang is intentionally small: assignments, references, namespaces, and
includes.

# limpid

A Rust utility for measuring and comparing binary sizes and build times between
different versions of [facet](https://github.com/facet-rs/facet)

It is mainly designed to compare pull requests with the main branch. It is meant
to run in CI and generate a Markdown report which is commented on the PR with
another GitHub Actions workflow.

## Usage

The file hierarchy is something like this:

```
facet/
  facet/
    Cargo.toml
  facet-core/
    Cargo.toml
  facet-reflect/
    Cargo.toml
limpid/
  limpid/
    Cargo.toml # this tool
  kitchensink/
    ks-facet/
      Cargo.toml
    ks-serde/
      Cargo.toml
    ks-facet-json-read/
    # etc.
```

Both `facet/` and `limpid/` are non-shallow clones of git repositories.

First, we'll do a build analysis using the substance API. This must be done
serially as it measures build times. We're building the `ks-facet` crate by
passing its manifest path.

Secondly, we'll create two git worktrees — one for `facet` and one for `limpid`.
For `limpid`, we'll just use whatever the current HEAD is, and for `facet` we'll
use the `main` branch. We'll `--detach` both of these, and do that in some
temporary directory that's somewhat stable, like `/tmp/limpid-main-workspace`.

The point is, `limpid/kitchensink/` has path dependencies with relative paths
that point to facet — and facet crates themselves have path dependencies amongst
themselves. By creating worktrees for _both_, and putting them next to each other
in the temp dir...

```
/tmp/
  limpid-main-workspace/
    facet/       # worktree from facet repo at main branch
      facet/
      facet-core/
      facet-reflect/
    limpid/      # worktree from limpid repo at current HEAD
      limpid/
      kitchensink/
        ks-facet/
        ks-serde/
        ks-facet-json-read/
        # etc.
```

...it should still build. Which allows us to run analysis builds for the
`ks-facet` binary again but using the `main` branch of facet.

Then we can use the substance API to do a differential analysis, and print the
results to the standard output.

Check out `llm.txt` in the `substance` dir for more information, and don't
forget to check the examples for example usage.

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

Both `facet/` and `kitchensink/` are non-shallow clones of git repositories.

First, we'll create two git worktrees — one for `facet` and one for `limpid` —
we'll create them with `HEAD` and `--detach`

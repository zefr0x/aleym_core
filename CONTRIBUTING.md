# Contributing

## Bug report

For now [GitHub Issue Traker](https://github.com/zefr0x/aleym_core/issues) is used for this project.

For security issues please read [`SECURITY.md`](./SECURITY.md).

## Development

- The latest `stable` [Rust](https://www.rust-lang.org/) programming language is used mainly in this project.
- Dependencies management is handled using [Cargo](https://doc.rust-lang.org/stable/cargo/).
- Building and installing are handled using [Meson](https://mesonbuild.com/)

### Branches

The following rules are enforced:

- As possible the `main` branch should always pass CI/CD without failures
- Merge Commits are forbidden in the `main` branch, Rebase must be used before merging into `main` to maintain a linear
  history
- Unless it make sense, a Squash Merge will be used when merging into `main` (aim for one goal when working in a Merge
  Request)

#### Naming

Use `<category>[/<reference>]/<description-in-kebab-case>` as a template (e.g. `refac/issue-404/deprecate-unused-api`,
`release/v2`, `release/v2.5`, etc.), just use alphanumeric characters with kebab case, and make sure that its short and
descriptive.

The enforced categories are the same as in commit types below, except that the `revert` type is excluded.

### Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org) specification for our commit messages, except
that a commit message is for developers and auditors, not to automatically generating CHANGELOGs for the public.

The following types are enforced:

- `feat`: New feature
- `fix`: Bug fix
- `refac`: Code change that neither fixes a bug nor adds a feature (e.g. improvement, deprecation, cleaning, etc.)
- `deps`: Adding, deleting, or updating external dependencies. If an update includes breaking changes, a separate
  `refac` direct child commit must be made to address those changes
- `sty`: Changes that do not affect the meaning of the code (e.g. white-space, formatting, simple lints, etc.)
- `lint`: Changes in linting or formatting rules, config, or tools
- `test`: Adding missing tests, correcting existing tests, or changes in the testing system
- `build`: Changes that affect the build system
- `cid`: Changes to our CI/CD configuration files and scripts
- `dev`: Changes to development helping scrips and tools
- `release`: New version (changes in changelogs and build system)
- `docs`: Documentation only changes (including code comments, some metadata in appstream file, etc.)
- `i18n`: Internationalization and localization changes (language code must be specified when adding or modifying
  translations)
- `data`: Changes to other data files attached to or used by the application either in run-time or compile-time (e.g.
  icons, etc.)
- `revert`: Reverts a previous commit

> [!TIP]
> Just read the commit history to see how to works.

### Style

To make every thing easy [**`pre-commit`**](https://pre-commit.com/) is used in this project, it should run in every
commit, so you shouldn't commit any thing without checking it first.

First install it, than add it as a git hook while you are inside the repository:

```shell
pre-commit install
```

#### Commit Messages

- `check_commit` is used to enforce rules written above

#### Rust

- `rustfmt` is used to enforce code formatting
- `Clippy` is used to enforce linting rules

#### Toml

- `Taplo` is used to enforce formatting

#### Markdown & JSON

- `Deno` is used to enforce formatting

<!-- TODO: ## Documentation -->

## Releases

## Versioning

The [Semantic Versioning](https://semver.org/) is used for version numbering in this project.

### What is a Release?

A release is a [git tag](https://git-scm.com/docs/git-tag) in the `main` branch of the project's git repository that
starts with the letter `v` followed by the Semantic Versioning based version number.

#### Creating a New Release

1. Increment the version number in the [`meson.build`](./meson.build) and [`Cargo.toml`](./Cargo.toml).
2. Add a new release in `CHANGELOG.md` files for libraries updates.
3. Add a new `<release>` tag under the `<releases>` tag in appstream files with information about the release that
   concerns the end user for applications and services updates.
4. Create a git commit for the release containing those changes (use the `release` commit type).
5. Create a signed git tag with a `v` letter followed by the version number (e.g. for `v1.5.3` you should do
   `git tag -s v1.5.3`).
6. Push changes to the remote using `git push origin main --tags`

> [!TIP]
> Just take a look at previous release commits.

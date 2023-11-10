## about

salt is a tool which acts as a simple interface for your project. You let the
users know what your project can do for them.

### Quickstart

#### Installing salt binary

Install the `salt` binary by doing

```
curl ...
```

## commands

- b - `cargo build` - builds salt binary
- i - `cp target/debug/salt /usr/local/bin/s` - installs the salt binary
- r - `cargo run` - runs the salt binary
- c - `cargo clippy` - runs clippy command for salt binary

## options

- type - project
- name - bin

## help

this project is the for salt interface

### Use Cases

#### Use cases of salt bundler

- Quickly create a CLI with 2 commands `s init` and `s pin`
- Importing scripts from anywhere. If your git repository has `salt.json` it is
  considered as a bundle.
- Can be used as a tooling framework for your project/organization.
- Cross-team tooling and collaboration
- No directory/context switching needed
- Seamless updates of your bundle through `s update`

### Pinning Existing Project

#### Pinning

You can create `SALT.md` in any folder and use it as a bundle using the `pin`
command

```sh
s pin
```

#### Unpinning

Pinned bundle can be unpinned with the following command

```sh
s unpin
```

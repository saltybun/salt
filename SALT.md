## about

salt is a tool which acts as a simple interface for your project. You let the
users know what your project can do for them.

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

<!-- documentation starts here -->

### Quickstart

#### Why 🧂

`salt` drastically reduces **_context-switching_** for developers and helps them
focus on the task. This tool provides all the convienience automating of
repeating tasks.

#### Installing salt binary

> Install `salt` bundler

```sh
sh https://github.com/codekidx/salt/salt.sh
```

> Add your first bundle

```sh
s add https://github.com/saltybun/salt-chips.git
```

> Run the new chips command

```sh
s chips
```

### Use Cases

#### Use cases of salt bundler

- Quickly create a CLI with 2 commands `s init` and `s pin`
- Importing scripts from anywhere. If your git repository has `salt.json` it is
  considered as a bundle.
- Can be used as a tooling framework for your project/organization.
- Cross-team tooling and collaboration
- No directory/context switching needed
- Seamless updates of your bundle through `s update`

### Intrinsics

#### Salt Commands

- `init` - inits a new `salt.json` file in the current directory with example
  command
- `add` - adds a bundle to your salt interface
- `pin` - pinning the folder as a salt bundle
- `unpin` - unpin a salt bundle
- `open` - open a bundle in your default file manager
- `jump` - jump to the bundle folder
- `watch {BUNDLE} {COMMAND}` - runs the command from your bundle and restarts
  the process if directory contents changes
- `+ {BUNDLE} {COMMAND...}` - wildcard command to run any command on a pinned
  bundle
- `-` - runs the last salt command

### Creating a new bundle

A new bundle can be initialized with the following command.

```sh
s init
```

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

### Jumping to a bundle

To switch between bundles efficiently salt has the `jump | j` command which can
be used with `cd` command to jump to the bundle directory.

```sh
cd $(s j $bundle)
```

where `$bundle` is the name of your bundle.

# 🧂

Salt lets you define bundles of your script and bring all your scripts under a
single command called `salt`.

### `salt` use-cases

- Simlest way to create your own CLI
- Simplest way to share your scripts to the world
- No BS framework to arrange your scripts across different projects
- Can be used as a collaborative tool to share scripts across teams

### `salt` quickstart

> Install salt interface

```sh
sh https://github.com/codekidx/salt/salt.sh
```

> Add your first bundle

```sh
salt add https://github.com/codekidx/salt-pixel.git
```

> Run the new pixel command

```sh
salt pixel
```

### `salt` intrinsics

- `add` - adds a bundle to your salt interface
- `i:{VALUE}` - install the package `VALUE`
- `watch:{BUNDLE:COMMAND}` - runs the command `salt BUNDLE COMMAND` and watches
  for file changes in the current directory

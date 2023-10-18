Salt is command line bundler and a command manager that lets you define bundles
of scripts and bring all your scripts under a single command called `salt | s`.
The focus of this tool is to seamlessly share scripts between teams and improve
productivity with ease of use. [Bundles](#salt-bundle) let your quickly import
scripts from any folder or a git repository. This project was initially created
for [🍿](https://github.com/codekidx/popcorn) and was extracted as a standalone
tool.

# 🧂

> _Boost your development workflow with a 🤏🏻 of 🧂_

![version](https://img.shields.io/badge/version-v0.1.0-orange)
![category](https://img.shields.io/badge/beta-teal)

## Getting Started

- [`salt` use-cases](#salt-use-cases)
- [`salt` quickstart](#salt-quickstart)
- [`salt` intrinsics](#salt-intrinsics)
- [`salt.json`](#saltjson)
  - [root fields](#root)
  - [command fields](#command)
- [`salt` bundle](#salt-bundle)
  - [creating a new bundle](#creating-a-new-bundle)
  - [pinning a folder as bundle](#pinning-a-folder-as-bundle)
  - [updating a bundle](#updating-a-bundle)

### `salt` use-cases

- Quickly create a CLI with 2 commands `s init` and `s pin`
- Importing scripts from anywhere. If your git repository has `salt.json` it is
  considered as a bundle.
- Can be used as a tooling framework for your project/organization.
- Cross-team tooling and collaboration
- No directory/context switching needed
- Seamless updates of your bundle through `s update`

### `salt` quickstart

> Install salt bundler

```sh
sh https://github.com/codekidx/salt/salt.sh
```

> Add your first bundle

```sh
s add https://github.com/codekidx/salt-popcorn.git
```

> Run the new popcorn command

```sh
s popcorn
```

### `salt` intrinsics

- `init` - inits a new `salt.json` file in the current directory with example
  command
- `add` - adds a bundle to your salt interface
- `update` - update all added bundles
- `pin` - pinning the folder as a salt bundle
- `watch {BUNDLE} {COMMAND}` - runs the command from your bundle and restarts
  the process if directory contents changes

<!-- - `install` - install the package `VALUE` -->

- `+ {BUNDLE} {COMMAND...}` - wildcard command to run any command on a pinned
  bundle
- `-` - runs the last salt command

### `salt.json`

```json
{
  "name": "do",
  "version": "1",
  "description": "my bundle of regularly used commands",
  "requires": [],
  "commands": {
    "ls": {
      "about": "list files in human readable form",
      "command": "ls",
      "args": ["-ahl"]
    }
  }
}
```

This is an example of salt bundle file. This file defines a command for salt
bundler to read and interpret a bundle called `do` and a command called `ls`.
The same can be invoked using the followin command:

```
s do ls
```

Here are the list of fields and descriptions of bundle file.

### root

| Field       | Description                                                  | Type                      |
| ----------- | ------------------------------------------------------------ | ------------------------- |
| name        | name of your bundle                                          | string                    |
| version     | version of your bundle, helps in updating a bundle           | string                    |
| description | description that is displayed in the help command            | string                    |
| requires    | defines a dependency on the list of packages for this bundle | `List<string>`            |
| commands    | a map of all commands in your bundle                         | `Object<string, command>` |

### command

| Field   | Description                                          | Type           |
| ------- | ---------------------------------------------------- | -------------- |
| about   | describing the working of this command               | string         |
| command | the command to invoke on your shell                  | string         |
| args    | the argument that needs to be passed to this command | `List<string>` |

### `salt` bundle

Salt bundles are a way of sharing your collection of script to another user.
Bundles are nothing but a git repository or a folder with [salt.json](#saltjson)
file.

### creating a new bundle

A new bundle can be initialized with the following command.

```sh
s init
```

### pinning a folder as bundle

You can create `salt.json` in any folder and use it as a bundle using the `pin`
command

```sh
s pin
```

### updating a bundle

If the bundle is added through a git repository (and not pinned) the bundler
will update the repository with the latest commits. To check for updates and
update to the latest version, you can run the `update` command

```sh
s update
```

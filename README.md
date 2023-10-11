# 🧂

Salt is command line bundler and a command aggregator that lets you define
bundles of scripts and bring all your scripts under a single command called
`salt`. The focus of this tool is to seamlessly share scripts between teams and
improve productivity and ease of use. Bundles lets your quickly import

## Documentation

- [`salt` use-cases](#salt-use-cases)
- [`salt` quickstart](#salt-quickstart)
- [`salt` intrinsics](#salt-intrinsics)
- [`salt.json`](#saltjson)

### `salt` use-cases

- Quickly create a CLI with 2 commands `salt init` and `salt sym`
- Importing scripts from anywhere. If your git repository has `salt.json` it is
  considered as a bundle.
- CAn be used as a tooling framework for your project/organization.
- Cross-team scripts collaboration tool

### `salt` quickstart

> Install salt bundler

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

- `init` - inits a new `salt.json` file in the current directory with example
  command
- `add` - adds a bundle to your salt interface
- `sym` - creates a symlink from the current folder to the bundle location
- `watch {BUNDLE} {COMMAND}` - runs the command `salt watch BUNDLE COMMAND` and
  watches for file changes in the current directory
- `i` - install the package `VALUE`

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

```sh
salt do ls
```

Here are the list of fields and descriptions of bundle file.

### root

| Field       | Description                                                  | Type                    |
| ----------- | ------------------------------------------------------------ | ----------------------- |
| name        | name of your bundle                                          | string                  |
| version     | version of your bundle, helps in updating a bundle           | string                  |
| description | description that is displayed in the help command            | string                  |
| requires    | defines a dependency on the list of packages for this bundle | List<string>            |
| commands    | a map of all commands in your bundle                         | Object<string, Command> |

### Command

| Field   | Description                                          | Type         |
| ------- | ---------------------------------------------------- | ------------ |
| about   | describing the working of this command               | string       |
| command | the command to invoke on your shell                  | string       |
| args    | the argument that needs to be passed to this command | List<string> |

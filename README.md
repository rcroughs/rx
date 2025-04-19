<div align="center">
<h1>rx</h1>
<p>A highly flexible file explorer on terminal</p>
</div>

## Features

- **File Explorer**: Navigate through your file system with ease.
- **File Operations**: Create, delete, and rename files and directories.
- **Undo-tree**: Keep track of your file operations and undo them if necessary.
- **Customizable**: Build your own plugins using the provided lua API.

## Installation

For the moment, only compiling from source is supported. You can do this by running the following commands:

```bash
git clone https://github.com/rcroughs/rx.git
cd rx
cargo build --release
```

The compiled binary will be located in `target/release/rx`. You can move it to a directory in your PATH for easier access.

## Usage
You can pass the following arguments to the binary:
 - `-h` or `--help`: Show the help message.
 - `-o` or `--output`: When you leave the app, it will write the current directory to stdout.

### Examples
 - Making an alias to jump easily to a directory:
```bash
alias rx='cd $(rx -o)'
```

## Scripting
In your config directory (`~/.config/rx`), you can create a `init.lua` file. This file will be loaded when the app starts. 
You can use this file to define your own plugins and customize the app to your liking.

Please note that the API is still in development and may change in the future. Moreover, the API is not documented yet, but you can find some examples in the `examples` directory.

## Contributing
Contributions are welcome! If you have any ideas or suggestions, feel free to open an issue or submit a pull request.

## License
This project is licensed under the GNU General Public License v3.0. See the [LICENSE](LICENSE) file for details.

## Acknowledgements
- [Rust](https://www.rust-lang.org/)
- [Lua](https://www.lua.org/)
- [crossterm](https://github.com/crossterm-rs/crossterm)
- [dirs](https://codeberg.org/dirs/dirs-rs)
- [chrono](https://github.com/chronotope/chrono)
- [toml](https://github.com/toml-rs/toml)
- [serde](https://github.com/serde-rs/serde)
- [clap](https://github.com/clap-rs/clap)
- [mlua](https://github.com/mlua-rs/mlua)
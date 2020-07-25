# Scripts

## `types_gen.py`

ChainX types generator helps you automatically generate 80% of the new types definition.

The idea is to retrieve `Enum`/`Struct`/`Type` Rust elements which are unknown to JS based on ctags. This script will also convert the fields of structs from `snake_underscore_case` to `camelCase`.

### Requirement

- Linux
- Python 3.6+
- https://github.com/universal-ctags/ctags
- https://github.com/sharkdp/fd
- The rust source files have been formatted using `rustfmt`.

### Limitations

- Can not handle the type generated from marcos due to the limination of ctags.
- Can not handle the unknown type that does not reported on JS side, e.g., nested `Struct`.
- Can not handle the type defined in the imported libraries.

### Run

Firstly, modify `NEW_TYPES` in `types_gen.py` if you find JS reports new unknown types.

```bash
$ cd scripts
$ ./types_gen.py
# See the generated files:
# res
# ├── chainx_rpc.json
# └── chainx_types.json
```

The auto generated `chainx_types.json` and `chainx_rpc.json` **need a review** to handle the corner cases because the script unevitably has some liminations.

If some types are failed to be extracted correctly using the script, then you just write them into `chainx_types_manual.json` or `chainx_rpc_manual.json` by hand. These manully created types will always override the auto generated ones.

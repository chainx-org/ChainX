# Scripts

## `types_gen.py`

ChainX types generator helps you automatically generate 80% of the new types definition.

The idea is to retrieve `Enum`/`Struct`/`Type` Rust elements which are unknown to JS based on ctags.

### Requirement

- Linux
- https://github.com/universal-ctags/ctags
- https://github.com/sharkdp/fd

### Limitations

- Can not handle the type generated from marcos due to the limination of ctags.
- Can not handle the unknown type that does not reported on JS side.

### Run

```bash
$ cd scripts
$ ./types_gen.py
# See the generated chainx_types.json
```

The auto generated `chainx_types.json` needs a review to handle the corner cases because this approach unevitably has some liminations.

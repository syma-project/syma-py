# syma-py

Python bindings for [Syma](https://github.com/tanganke/syma) — a symbolic-first programming language inspired by Wolfram Language.

## Installation

```bash
pip install maturin
maturin develop  # from the syma-py/ directory
```

## Usage

```python
import syma_py

kernel = syma_py.SymaKernel()

# Arithmetic — returns native Python types
kernel.eval("1 + 2")           # → 3
kernel.eval("2^10")            # → 1024
kernel.eval("{1, 2, 3}")       # → [1, 2, 3]
kernel.eval('"hello"')         # → "hello"
kernel.eval("<|a -> 1|>")      # → {"a": 1}

# Symbolic computation — returns SymaValue wrapper
kernel.eval("Expand[(x+y)^3]")
# → SymaValue(type='call', display='x^3 + 3*x^2*y + 3*x*y^2 + y^3')

kernel.eval("Integrate[x^2, x]")
# → SymaValue(type='call', display='x^3/3')

# Stateful environment
kernel.eval("f[x_] := x^2 + 1")
kernel.eval("f[5]")            # → 26

kernel.set("x", 42)
kernel.get("x")                # → 42

# Detailed evaluation info
kernel.eval_detailed("1+2")
# → {"success": True, "results": [...], "timing_ms": 0, "messages": []}
```

## API

### `SymaKernel()`

| Method | Returns | Description |
|---|---|---|
| `eval(code)` | `int`, `float`, `str`, `bool`, `list`, `dict`, or `SymaValue` | Evaluate Syma code, return last result |
| `eval_raw(code)` | same as `eval()` | Evaluate and return raw value |
| `eval_detailed(code)` | `dict` | Full structured result with timing |
| `get(name)` | converted value or `None` | Get variable from environment |
| `set(name, value)` | `None` | Set variable in environment |
| `bindings()` | `dict` | All environment bindings |

### `SymaValue`

Wrapper for Syma types without a native Python equivalent.

| Attribute/Method | Description |
|---|---|
| `.type_tag` | Type tag string (e.g. `"sym"`, `"call"`, `"func"`) |
| `.display` | Syma display string |
| `str(val)` | Same as `.display` |
| `.to_python()` | Best-effort native conversion |
| `.to_json()` | Raw tagged-JSON representation |
| `.value` | Inner value portion of the tagged JSON |

## Value Conversion

| Syma Type | Python Type | Direction |
|---|---|---|
| `Integer` | `int` | both ways |
| `Real` | `float` | both ways |
| `Str` | `str` | both ways |
| `Bool` | `bool` | both ways |
| `Null` | `None` | both ways |
| `List` | `list` | both ways |
| `Assoc` | `dict` | both ways |
| Symbol, Call, Function, ... | `SymaValue` | output only |

## Development

```bash
cd syma-py
maturin develop              # build + install
python -m pytest -v          # run tests (35 tests)
```

## License

MIT

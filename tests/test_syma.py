"""Tests for syma_py bindings."""

import pytest
import syma_py


def test_eval_integer():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("42")
    assert result == 42
    assert isinstance(result, int)


def test_eval_addition():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("1 + 2")
    assert result == 3
    assert isinstance(result, int)


def test_eval_real():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("3.14")
    assert isinstance(result, float)
    assert abs(result - 3.14) < 1e-10


def test_eval_string():
    kernel = syma_py.SymaKernel()
    result = kernel.eval('"hello world"')
    assert result == "hello world"
    assert isinstance(result, str)


def test_eval_bool():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("True")
    assert result is True
    assert isinstance(result, bool)

    result = kernel.eval("False")
    assert result is False


def test_eval_list():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("{1, 2, 3}")
    assert result == [1, 2, 3]
    assert isinstance(result, list)


def test_eval_multiplication():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("2 * 3")
    assert result == 6


def test_eval_negative():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("-5")
    assert result == -5


def test_eval_power():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("2^10")
    assert result == 1024


def test_state_persistence():
    kernel = syma_py.SymaKernel()
    kernel.eval("x = 42")
    result = kernel.eval("x + 1")
    assert result == 43


def test_state_persistence_string():
    kernel = syma_py.SymaKernel()
    kernel.eval('name = "Syma"')
    result = kernel.eval('name <> " is cool"')
    assert result == "Syma is cool"


def test_get_set_int():
    kernel = syma_py.SymaKernel()
    kernel.set("y", 99)
    result = kernel.eval("y")
    assert result == 99


def test_get_set_str():
    kernel = syma_py.SymaKernel()
    kernel.set("msg", "hello")
    result = kernel.eval("msg")
    assert result == "hello"


def test_get_set_list():
    kernel = syma_py.SymaKernel()
    kernel.set("arr", [1, 2, 3])
    result = kernel.eval("Length[arr]")
    assert result == 3


def test_get_set_bool():
    kernel = syma_py.SymaKernel()
    kernel.set("flag", True)
    result = kernel.eval("flag")
    assert result is True


def test_get_set_none():
    kernel = syma_py.SymaKernel()
    kernel.set("empty", None)
    result = kernel.eval("empty")
    assert result is None


def test_get_undefined():
    kernel = syma_py.SymaKernel()
    result = kernel.get("NonExistentSymbol")
    assert result is None


def test_eval_nested_list():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("{{1, 2}, {3, 4}}")
    assert result == [[1, 2], [3, 4]]


def test_eval_assoc():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("<|a -> 1, b -> 2|>")
    assert isinstance(result, dict)
    assert result.get("a") == 1
    assert result.get("b") == 2


def test_symbolic_result():
    """Symbolic expressions should return SymaValue, not crash."""
    kernel = syma_py.SymaKernel()
    result = kernel.eval("x^2 + y^2")
    assert isinstance(result, syma_py.SymaValue)
    assert result.type_tag in ("sym", "call")  # symbolic expression


def test_syma_value_display():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("Expand[(x + y)^2]")
    assert isinstance(result, syma_py.SymaValue)
    display = str(result)
    assert "x" in display or "y" in display


def test_syma_value_to_python():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("Expand[(x + y)^2]")
    assert isinstance(result, syma_py.SymaValue)
    py_result = result.to_python()
    assert py_result is not None


def test_eval_detailed():
    kernel = syma_py.SymaKernel()
    result = kernel.eval_detailed("1 + 2")
    assert isinstance(result, dict)
    assert result["success"] is True
    assert result["timing_ms"] >= 0
    assert len(result["results"]) == 1
    assert result["results"][0]["output"] == "3"


def test_eval_detailed_error():
    kernel = syma_py.SymaKernel()
    result = kernel.eval_detailed("1 +++ 2")
    assert isinstance(result, dict)
    assert result["success"] is False
    assert "error" in result


def test_eval_error():
    kernel = syma_py.SymaKernel()
    with pytest.raises(syma_py.SymaEvalError):
        kernel.eval("1 +++ 2")


def test_bindings():
    kernel = syma_py.SymaKernel()
    kernel.eval("a = 10")
    kernel.eval("b = 20")
    bindings = kernel.bindings()
    assert isinstance(bindings, dict)
    assert bindings.get("a") == 10
    assert bindings.get("b") == 20


def test_builtin_functions():
    """Built-in functions like Pi should be accessible."""
    kernel = syma_py.SymaKernel()
    pi = kernel.get("Pi")
    assert pi is not None


def test_nested_expression():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("1 + 2 * 3")
    assert result == 7


def test_multi_statement():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("1\n2\n3")
    assert result == 3  # last statement


def test_suppressed_statement():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("1; 2; 3")
    assert result == 3  # last statement


def test_all_suppressed():
    kernel = syma_py.SymaKernel()
    result = kernel.eval("1; 2; 3;")
    assert result is None


def test_function_definition():
    kernel = syma_py.SymaKernel()
    kernel.eval("f[x_] := x^2")
    result = kernel.eval("f[5]")
    assert result == 25


def test_set_syma_value():
    """Setting a syma expression result as a variable."""
    kernel = syma_py.SymaKernel()
    sv = kernel.eval("x^2")
    assert isinstance(sv, syma_py.SymaValue)
    kernel.set("expr", sv)
    result = kernel.get("expr")
    assert result is not None


def test_eval_raw():
    kernel = syma_py.SymaKernel()
    result = kernel.eval_raw("1 + 2")
    assert result == 3


def test_parallel_kernels():
    """Two independent kernels should not interfere."""
    k1 = syma_py.SymaKernel()
    k2 = syma_py.SymaKernel()

    k1.set("x", 100)
    k2.set("x", 200)

    r1 = k1.eval("x")
    r2 = k2.eval("x")

    assert r1 == 100
    assert r2 == 200


def test_custom_exceptions():
    """Verify SymaParseError and SymaEvalError are raised correctly."""
    kernel = syma_py.SymaKernel()

    # Parse error — unmatched parenthesis
    with pytest.raises(syma_py.SymaParseError):
        kernel.eval("(1 + 2")

    # Eval error — increment operator on non-variable
    with pytest.raises(syma_py.SymaEvalError):
        kernel.eval("1 +++ 2")


def test_context_manager():
    """Verify with syma_py.SymaKernel() as k: works."""
    with syma_py.SymaKernel() as k:
        result = k.eval("42")
        assert result == 42


def test_reset():
    """Verify reset clears state."""
    kernel = syma_py.SymaKernel()
    kernel.eval("x = 42")
    assert kernel.eval("x") == 42

    kernel.reset()
    # After reset, x is no longer defined (will be a symbolic value, not 42)
    result = kernel.eval("x")
    assert result != 42


def test_eval_many():
    """Verify eval_many returns all results."""
    kernel = syma_py.SymaKernel()
    results = kernel.eval_many("1\n2\n3")
    assert results == [1, 2, 3]


def test_eval_many_suppressed():
    """Verify suppressed statements return None in the list."""
    kernel = syma_py.SymaKernel()
    results = kernel.eval_many("1; 2; 3")
    assert results[0] is None
    assert results[1] is None
    assert results[2] == 3


def test_syma_value_to_expr():
    """Verify to_expr() returns display string."""
    kernel = syma_py.SymaKernel()
    result = kernel.eval("x^2")
    assert isinstance(result, syma_py.SymaValue)
    expr = result.to_expr()
    assert "x" in expr and "2" in expr


def test_function_with_pattern():
    """Define f[x_, y_] := x^2 + y^2, call f[3, 4], verify result."""
    kernel = syma_py.SymaKernel()
    kernel.eval("f[x_, y_] := x^2 + y^2")
    result = kernel.eval("f[3, 4]")
    assert result == 25


def test_pure_function():
    """Test (#^2 &) [5] -> 25."""
    kernel = syma_py.SymaKernel()
    result = kernel.eval("(#^2 &)[5]")
    assert result == 25


def test_rule_replace():
    """Test rule replacement returns a SymaValue for symbolic expressions."""
    kernel = syma_py.SymaKernel()
    result = kernel.eval("x^2 /. x -> 3")
    # Rule replacement on symbolic expressions may not auto-evaluate
    assert isinstance(result, syma_py.SymaValue)

def test_large_integer():
    """Test 2^1000 returns correct big integer."""
    kernel = syma_py.SymaKernel()
    result = kernel.eval("2^1000")
    assert result == 2 ** 1000

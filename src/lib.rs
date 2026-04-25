#![allow(deprecated)]  // PyO3 0.23 uses deprecated IntoPy (replaced by IntoPyObject in 0.24)

/// syma-py: PyO3 bindings for the Syma symbolic programming language.
///
/// Exposes SymaKernel (eval, get, set) and SymaValue wrapper to Python.
use std::collections::HashMap;
use std::ffi::CString;

use pyo3::exceptions::{PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::Value as JValue;

use syma::ffi::marshal::{json_val_to_value_full, value_to_json_full};
use syma::kernel::{StatementResult, SymaKernel};
use syma::value::Value;

// ═══════════════════════════════════════════════════════════════════════════════
// SymaValue — Python wrapper for non-native Syma types
// ═══════════════════════════════════════════════════════════════════════════════

/// Wraps a Syma value that doesn't have a native Python equivalent
/// (e.g. Symbol, Call, Function, Rule, etc.).
#[pyclass(name = "SymaValue", module = "syma_py")]
#[derive(Clone)]
struct PySymaValue {
    /// Type tag from tagged JSON (e.g. "sym", "call", "func", "rule").
    #[pyo3(get)]
    type_tag: String,
    /// The full tagged JSON representation.
    json_value: serde_json::Value,
    /// Syma display string (e.g. "x^2 + y^2").
    #[pyo3(get)]
    display: String,
}

#[pymethods]
impl PySymaValue {
    fn __str__(&self) -> String {
        self.display.clone()
    }

    fn __repr__(&self) -> String {
        format!("SymaValue(type='{}', display='{}')", self.type_tag, self.display)
    }

    /// Best-effort conversion to a native Python object.
    fn to_python(&self, py: Python<'_>) -> PyObject {
        json_value_to_python(&self.json_value, py)
    }

    /// Return the raw tagged-JSON representation as a Python dict.
    fn to_json(&self, py: Python<'_>) -> PyObject {
        serde_json_value_to_pyobject(&self.json_value, py)
    }

    /// The inner value portion of the tagged JSON (the "v" field), if present.
    #[getter]
    fn value(&self, py: Python<'_>) -> PyObject {
        if let Some(obj) = self.json_value.as_object() {
            if let Some(v) = obj.get("v") {
                return serde_json_value_to_pyobject(v, py);
            }
        }
        serde_json_value_to_pyobject(&self.json_value, py)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SymaKernel — Python class wrapping the Syma evaluator
// ═══════════════════════════════════════════════════════════════════════════════

/// Syma evaluation engine — holds state across calls.
///
/// Usage:
///     from syma_py import SymaKernel
///     k = SymaKernel()
///     result = k.eval("1 + 2")  # → 3
#[pyclass(name = "SymaKernel", module = "syma_py")]
struct PySymaKernel {
    kernel: SymaKernel,
}

#[pymethods]
impl PySymaKernel {
    #[new]
    fn new() -> Self {
        PySymaKernel {
            kernel: SymaKernel::new(),
        }
    }

    /// Evaluate Syma code and return the last result.
    ///
    /// Common types (int, float, str, bool, None, list, dict) are converted
    /// to native Python objects. Other types are returned as SymaValue.
    ///
    /// Raises RuntimeError on parse/eval errors.
    fn eval(&self, py: Python<'_>, code: &str) -> PyResult<PyObject> {
        let kr = self.kernel.eval(code);

        if !kr.success {
            let msg = kr.error.unwrap_or_else(|| "Evaluation failed".to_string());
            return Err(PyRuntimeError::new_err(msg));
        }

        let results = match kr.results {
            Some(r) => r,
            None => return Ok(py.None()),
        };

        // Return the last non-None result
        for stmt in results.into_iter().rev() {
            if let Some(stmt_result) = stmt {
                return Ok(stmt_result_to_python(stmt_result, py));
            }
        }

        Ok(py.None())
    }

    /// Evaluate Syma code and return the raw Value directly.
    ///
    /// Unlike eval(), this always returns either a native Python type or
    /// a SymaValue wrapper — never a structured result dict.
    fn eval_raw(&self, py: Python<'_>, code: &str) -> PyResult<PyObject> {
        match self.kernel.eval_raw(code) {
            Ok(val) => Ok(value_to_python(&val, py)),
            Err(e) => Err(PyRuntimeError::new_err(e.to_string())),
        }
    }

    /// Evaluate Syma code and return the full structured result as a dict.
    ///
    /// Returns a dict with keys:
    ///   success (bool), results (list of dicts or None),
    ///   messages (list of str), error (str or None), timing_ms (int)
    fn eval_detailed(&self, py: Python<'_>, code: &str) -> PyObject {
        let kr = self.kernel.eval(code);
        kernel_result_to_python(kr, py)
    }

    /// Get a variable from the Syma environment.
    ///
    /// Returns None if the variable is not defined.
    fn get(&self, py: Python<'_>, name: &str) -> PyObject {
        let env = self.kernel.env();
        match env.get(name) {
            Some(val) => value_to_python(&val, py),
            None => py.None(),
        }
    }

    /// Set a variable in the Syma environment.
    ///
    /// Accepts: int, float, str, bool, None, list, dict, SymaValue.
    fn set(&self, name: &str, value: Bound<'_, PyAny>, py: Python<'_>) -> PyResult<()> {
        let val = python_to_syma_value(&value, py)?;
        self.kernel.env().set(name.to_string(), val);
        Ok(())
    }

    /// Get all bindings from the Syma environment as a dict.
    fn bindings(&self, py: Python<'_>) -> PyResult<PyObject> {
        let env = self.kernel.env();
        let bindings = env.all_bindings();
        let dict = PyDict::new(py);
        for (name, val) in &bindings {
            dict.set_item(name.as_str(), value_to_python(val, py))?;
        }
        Ok(dict.into())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PyO3 module registration
// ═══════════════════════════════════════════════════════════════════════════════

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySymaKernel>()?;
    m.add_class::<PySymaValue>()?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Value conversion: Syma ↔ Python
// ═══════════════════════════════════════════════════════════════════════════════

/// Convert a tagged-JSON Value to a Python object with native type extraction.
fn json_value_to_python<'py>(jv: &serde_json::Value, py: Python<'py>) -> PyObject {
    match jv {
        JValue::Null => py.None(),
        JValue::Bool(b) => b.into_py(py),
        JValue::Number(n) => n.as_f64().unwrap_or(0.0).into_py(py),
        JValue::String(s) => s.into_py(py),
        JValue::Array(arr) => {
            let items: Vec<PyObject> =
                arr.iter().map(|v| json_value_to_python(v, py)).collect();
            PyList::new(py, &items).unwrap().into()
        }
        JValue::Object(obj) => {
            let tag = obj.get("t").and_then(|v| v.as_str());
            match tag {
                Some("int") => {
                    let s = obj.get("v").and_then(|v| v.as_str()).unwrap_or("0");
                    // Fast path for common i64 range
                    if let Ok(n) = s.parse::<i64>() {
                        return n.into_py(py);
                    }
                    // Arbitrary precision via Python's int()
                    let code = CString::new(format!("int('{}')", s)).unwrap();
                    match py.eval(&code, None, None) {
                        Ok(v) => v.into_py(py),
                        Err(_) => py.None(),
                    }
                }
                Some("real") => {
                    obj.get("v")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0)
                        .into_py(py)
                }
                Some("str") => {
                    obj.get("v")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into_py(py)
                }
                Some("bool") => {
                    obj.get("v")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                        .into_py(py)
                }
                Some("null") => py.None(),
                Some("list") => {
                    if let Some(arr) = obj.get("v").and_then(|v| v.as_array()) {
                        let items: Vec<PyObject> =
                            arr.iter().map(|v| json_value_to_python(v, py)).collect();
                        PyList::new(py, &items).unwrap().into()
                    } else {
                        py.None()
                    }
                }
                Some("assoc") => {
                    if let Some(map) = obj.get("v").and_then(|v| v.as_object()) {
                        let dict = PyDict::new(py);
                        for (k, v) in map {
                            dict.set_item(k.as_str(), json_value_to_python(v, py))
                                .unwrap();
                        }
                        dict.into()
                    } else {
                        py.None()
                    }
                }
                // Complex types (`sym`, `call`, `func`, `rule`, etc.) — these
                // should be wrapped in SymaValue by callers; but if one reaches
                // this function directly, return the raw JSON dict.
                _ => serde_json_value_to_pyobject(jv, py),
            }
        }
    }
}

/// Convert a raw serde_json::Value to a Python object (no Syma type awareness).
fn serde_json_value_to_pyobject<'py>(jv: &serde_json::Value, py: Python<'py>) -> PyObject {
    match jv {
        JValue::Null => py.None(),
        JValue::Bool(b) => b.into_py(py),
        JValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py(py)
            } else if let Some(f) = n.as_f64() {
                f.into_py(py)
            } else {
                py.None()
            }
        }
        JValue::String(s) => s.into_py(py),
        JValue::Array(arr) => {
            let items: Vec<PyObject> =
                arr.iter().map(|v| serde_json_value_to_pyobject(v, py)).collect();
            PyList::new(py, &items).unwrap().into()
        }
        JValue::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k.as_str(), serde_json_value_to_pyobject(v, py))
                    .unwrap();
            }
            dict.into()
        }
    }
}

/// Convert a Syma StatementResult to a Python object.
fn stmt_result_to_python(stmt: StatementResult, py: Python<'_>) -> PyObject {
    let display = stmt.output;
    let jv = stmt.value;

    if should_wrap_in_syma_value(&jv) {
        let type_tag = jv
            .as_object()
            .and_then(|o| o.get("t").and_then(|v| v.as_str()))
            .unwrap_or("unknown")
            .to_string();
        let sv = PySymaValue {
            type_tag,
            json_value: jv,
            display,
        };
        Py::new(py, sv).unwrap().into_py(py)
    } else {
        json_value_to_python(&jv, py)
    }
}

/// Convert a Rust Value to a Python object.
fn value_to_python(val: &Value, py: Python<'_>) -> PyObject {
    let jv = value_to_json_full(val);

    if should_wrap_in_syma_value(&jv) {
        let type_tag = jv
            .as_object()
            .and_then(|o| o.get("t").and_then(|v| v.as_str()))
            .unwrap_or("unknown")
            .to_string();
        let display = val.to_string();
        let sv = PySymaValue {
            type_tag,
            json_value: jv,
            display,
        };
        Py::new(py, sv).unwrap().into_py(py)
    } else {
        json_value_to_python(&jv, py)
    }
}

/// Return true if a tagged-JSON value represents a non-native Syma type
/// (i.e., should be wrapped in SymaValue rather than converted to native Python).
fn should_wrap_in_syma_value(jv: &serde_json::Value) -> bool {
    if let Some(obj) = jv.as_object() {
        let tag = obj.get("t").and_then(|v| v.as_str());
        !matches!(
            tag,
            Some("int" | "real" | "str" | "bool" | "null" | "list" | "assoc")
        )
    } else {
        false
    }
}

/// Convert a KernelResult into a Python dict.
fn kernel_result_to_python(kr: syma::kernel::KernelResult, py: Python<'_>) -> PyObject {
    let dict = PyDict::new(py);
    dict.set_item("success", kr.success).unwrap();
    dict.set_item("timing_ms", kr.timing_ms).unwrap();

    if let Some(results) = kr.results {
        let py_results: Vec<PyObject> = results
            .into_iter()
            .map(|opt_stmt| {
                let d = PyDict::new(py);
                match opt_stmt {
                    Some(stmt) => {
                        d.set_item("output", stmt.output.as_str()).unwrap();
                        // Always convert via JSON for display; don't wrap in SymaValue
                        // since the caller asked for detailed/structured output.
                        d.set_item("value", json_value_to_python(&stmt.value, py))
                            .unwrap();
                    }
                    None => {
                        d.set_item("output", py.None()).unwrap();
                        d.set_item("value", py.None()).unwrap();
                    }
                }
                d.into()
            })
            .collect();
        dict.set_item("results", PyList::new(py, &py_results).unwrap())
            .unwrap();
    }

    dict.set_item("messages", kr.messages).unwrap();

    if let Some(err) = kr.error {
        dict.set_item("error", err).unwrap();
    }

    dict.into()
}

/// Convert a Python object to a Syma Value.
fn python_to_syma_value(obj: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Value> {
    // SymaValue → extract JSON and deserialize
    if let Ok(sv) = obj.extract::<PyRef<'_, PySymaValue>>() {
        return json_val_to_value_full(&sv.json_value)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()));
    }

    // None → Null
    if obj.is_none() {
        return Ok(Value::Null);
    }

    // Bool (must check before int, since Python bool is int subclass)
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Value::Bool(b));
    }

    // Int (i64 fast path)
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::Integer(rug::Integer::from(i)));
    }

    // Int (arbitrary precision via Python int → string)
    let is_py_int = obj.is_instance(&py.get_type::<pyo3::types::PyInt>())?;
    if is_py_int {
        let s = obj.str()?.to_string();
        if let Ok(n) = rug::Integer::parse(&s) {
            return Ok(Value::Integer(rug::Integer::from(n)));
        }
    }

    // Float
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(Value::Real(rug::Float::with_val(53, f)));
    }

    // String
    if let Ok(s) = obj.extract::<String>() {
        return Ok(Value::Str(s));
    }

    // List
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(python_to_syma_value(&item, py)?);
        }
        return Ok(Value::List(items));
    }

    // Dict → Assoc
    if let Ok(d) = obj.downcast::<PyDict>() {
        let mut map = HashMap::new();
        for (k, v) in d.iter() {
            let key = k.extract::<String>().map_err(|_| {
                PyTypeError::new_err("Dictionary keys must be strings for Syma Assoc")
            })?;
            map.insert(key, python_to_syma_value(&v, py)?);
        }
        return Ok(Value::Assoc(map));
    }

    Err(PyTypeError::new_err(format!(
        "Unsupported type '{}' for Syma value conversion",
        obj.get_type().name()?
    )))
}

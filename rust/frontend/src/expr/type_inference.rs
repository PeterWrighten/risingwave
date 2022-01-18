//! This type inference is just to infer the return type of function calls, and make sure the
//! functionCall expressions have same input type requirement and return type definition as backend.
use std::collections::HashMap;
use std::vec;

use itertools::iproduct;
use risingwave_common::types::DataTypeKind;
use risingwave_pb::expr::expr_node;

/// Infer the return type of a function. If the backend's expression implementation can't receive
/// the datatypes, return Null.
pub fn infer_type(
    func_type: expr_node::Type,
    inputs_type: Vec<DataTypeKind>,
) -> Option<DataTypeKind> {
    // With our current simplified type system, where all types are nullable and not parameterized
    // by things like length or precision, the inference can be done with a map lookup.
    infer_type_name(func_type, inputs_type)
}

/// Infer the return type name without parameters like length or precision.
fn infer_type_name(
    func_type: expr_node::Type,
    inputs_type: Vec<DataTypeKind>,
) -> Option<DataTypeKind> {
    FUNC_SIG_MAP
        .get(&FuncSign {
            func: func_type,
            inputs_type,
        })
        .cloned()
}

#[derive(PartialEq, Hash)]
struct FuncSign {
    func: expr_node::Type,
    inputs_type: Vec<DataTypeKind>,
}
impl Eq for FuncSign {}
#[allow(dead_code)]
impl FuncSign {
    pub fn new(func: expr_node::Type, inputs_type: Vec<DataTypeKind>) -> Self {
        FuncSign { func, inputs_type }
    }
    pub fn new_no_input(func: expr_node::Type) -> Self {
        FuncSign {
            func,
            inputs_type: vec![],
        }
    }
    pub fn new_unary(func: expr_node::Type, p1: DataTypeKind) -> Self {
        FuncSign {
            func,
            inputs_type: vec![p1],
        }
    }
    pub fn new_binary(func: expr_node::Type, p1: DataTypeKind, p2: DataTypeKind) -> Self {
        FuncSign {
            func,
            inputs_type: vec![p1, p2],
        }
    }
    pub fn new_ternary(
        func: expr_node::Type,
        p1: DataTypeKind,
        p2: DataTypeKind,
        p3: DataTypeKind,
    ) -> Self {
        FuncSign {
            func,
            inputs_type: vec![p1, p2, p3],
        }
    }
}
fn arithmetic_type_derive(t1: DataTypeKind, t2: DataTypeKind) -> DataTypeKind {
    if t2 as i32 > t1 as i32 {
        t2
    } else {
        t1
    }
}
fn build_type_derive_map() -> HashMap<FuncSign, DataTypeKind> {
    let mut map = HashMap::new();
    let num_types = vec![
        DataTypeKind::Int16,
        DataTypeKind::Int32,
        DataTypeKind::Int64,
        DataTypeKind::Float32,
        DataTypeKind::Float64,
        DataTypeKind::Decimal,
    ];
    let all_types = vec![
        DataTypeKind::Int16,
        DataTypeKind::Int32,
        DataTypeKind::Int64,
        DataTypeKind::Float32,
        DataTypeKind::Float64,
        DataTypeKind::Boolean,
        DataTypeKind::Char,
        DataTypeKind::Varchar,
        DataTypeKind::Decimal,
        DataTypeKind::Time,
        DataTypeKind::Timestamp,
        DataTypeKind::Interval,
        DataTypeKind::Date,
        DataTypeKind::Timestampz,
    ];
    let atm_exprs = vec![
        expr_node::Type::Add,
        expr_node::Type::Subtract,
        expr_node::Type::Multiply,
        expr_node::Type::Divide,
        expr_node::Type::Modulus,
    ];

    let cmp_exprs = vec![
        expr_node::Type::Equal,
        expr_node::Type::NotEqual,
        expr_node::Type::LessThan,
        expr_node::Type::LessThanOrEqual,
        expr_node::Type::GreaterThan,
        expr_node::Type::GreaterThanOrEqual,
    ];
    let logical_exprs = vec![
        expr_node::Type::And,
        expr_node::Type::Or,
        expr_node::Type::Not,
    ];
    let bool_check_exprs = vec![
        expr_node::Type::IsTrue,
        expr_node::Type::IsNotTrue,
        expr_node::Type::IsFalse,
        expr_node::Type::IsNotFalse,
    ];
    let null_check_exprs = vec![
        expr_node::Type::IsNull,
        expr_node::Type::IsNotNull,
        expr_node::Type::StreamNullByRowCount,
    ];

    for (expr, t1, t2) in iproduct!(atm_exprs, num_types.clone(), num_types.clone()) {
        map.insert(
            FuncSign::new_binary(expr, t1, t2),
            arithmetic_type_derive(t1, t2),
        );
    }
    for (expr, t1, t2) in iproduct!(cmp_exprs.clone(), num_types.clone(), num_types) {
        map.insert(FuncSign::new_binary(expr, t1, t2), DataTypeKind::Boolean);
    }
    for expr in cmp_exprs {
        map.insert(
            FuncSign::new_binary(expr, DataTypeKind::Boolean, DataTypeKind::Boolean),
            DataTypeKind::Boolean,
        );
    }
    for expr in logical_exprs {
        map.insert(
            FuncSign::new_binary(expr, DataTypeKind::Boolean, DataTypeKind::Boolean),
            DataTypeKind::Boolean,
        );
    }
    for expr in bool_check_exprs {
        map.insert(
            FuncSign::new_binary(expr, DataTypeKind::Boolean, DataTypeKind::Boolean),
            DataTypeKind::Boolean,
        );
    }
    for (expr, t) in iproduct!(null_check_exprs, all_types) {
        map.insert(FuncSign::new_unary(expr, t), DataTypeKind::Boolean);
    }

    map
}
lazy_static::lazy_static! {
  static ref FUNC_SIG_MAP: HashMap<FuncSign, DataTypeKind> = {
    build_type_derive_map()
  };
}
#[cfg(test)]
mod tests {
    use itertools::iproduct;
    use risingwave_common::types::DataTypeKind;
    use risingwave_pb::expr::expr_node;

    use super::infer_type;

    fn test_simple_infer_type(
        func_type: expr_node::Type,
        inputs_type: Vec<DataTypeKind>,
        expected_type_name: DataTypeKind,
    ) {
        let ret = infer_type(func_type, inputs_type).unwrap();
        assert_eq!(ret, expected_type_name);
    }
    fn test_infer_type_not_exist(func_type: expr_node::Type, inputs_type: Vec<DataTypeKind>) {
        let ret = infer_type(func_type, inputs_type);
        assert_eq!(ret, None);
    }

    #[test]
    fn test_arithmetics() {
        use DataTypeKind::*;
        let atm_exprs = vec![
            expr_node::Type::Add,
            expr_node::Type::Subtract,
            expr_node::Type::Multiply,
            expr_node::Type::Divide,
            expr_node::Type::Modulus,
        ];
        let num_promote_table = vec![
            (Int16, Int16, Int16),
            (Int16, Int32, Int32),
            (Int16, Int64, Int64),
            (Int16, Float32, Float32),
            (Int16, Float64, Float64),
            (Int16, Decimal, Decimal),
            (Int32, Int16, Int32),
            (Int32, Int32, Int32),
            (Int32, Int64, Int64),
            (Int32, Float32, Float32),
            (Int32, Float64, Float64),
            (Int32, Decimal, Decimal),
            (Int64, Int16, Int64),
            (Int64, Int32, Int64),
            (Int64, Int64, Int64),
            (Int64, Float32, Float32),
            (Int64, Float64, Float64),
            (Int64, Decimal, Decimal),
            (Float32, Int16, Float32),
            (Float32, Int32, Float32),
            (Float32, Int64, Float32),
            (Float32, Float32, Float32),
            (Float32, Float64, Float64),
            (Float32, Decimal, Decimal),
            (Float64, Int16, Float64),
            (Float64, Int32, Float64),
            (Float64, Int64, Float64),
            (Float64, Float32, Float64),
            (Float64, Float64, Float64),
            (Float64, Decimal, Decimal),
            (Decimal, Int16, Decimal),
            (Decimal, Int32, Decimal),
            (Decimal, Int64, Decimal),
            (Decimal, Float32, Decimal),
            (Decimal, Float64, Decimal),
            (Decimal, Decimal, Decimal),
        ];
        for (expr, (t1, t2, tr)) in iproduct!(atm_exprs, num_promote_table) {
            test_simple_infer_type(expr, vec![t1, t2], tr);
        }
    }

    #[test]
    fn test_bool_num_not_exist() {
        let exprs = vec![
            expr_node::Type::Add,
            expr_node::Type::Subtract,
            expr_node::Type::Multiply,
            expr_node::Type::Divide,
            expr_node::Type::Modulus,
            expr_node::Type::Equal,
            expr_node::Type::NotEqual,
            expr_node::Type::LessThan,
            expr_node::Type::LessThanOrEqual,
            expr_node::Type::GreaterThan,
            expr_node::Type::GreaterThanOrEqual,
            expr_node::Type::And,
            expr_node::Type::Or,
            expr_node::Type::Not,
        ];
        let num_types = vec![
            DataTypeKind::Int16,
            DataTypeKind::Int32,
            DataTypeKind::Int64,
            DataTypeKind::Float32,
            DataTypeKind::Float64,
            DataTypeKind::Decimal,
        ];

        for (expr, num_t) in iproduct!(exprs, num_types) {
            test_infer_type_not_exist(expr, vec![num_t, DataTypeKind::Boolean]);
        }
    }
}

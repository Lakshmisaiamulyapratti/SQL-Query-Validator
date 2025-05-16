use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use sqlparser::ast::{Expr, SelectItem, SetExpr, Statement};
use std::collections::HashMap;
use std::io::{self, Write};
use maplit::hashmap;

type Row = HashMap<String, String>;

#[allow(dead_code)]
struct Table {
    #[allow(dead_code)]
    name: String,
    rows: Vec<Row>,
}

// Evaluate the 'WHERE' condition for a given row recursivey by handling the logical operators
fn evaluate_condition(expr: &Expr, row: &Row) -> bool {
    match expr {
        // Handle binary operations like 'column = value' or 'condition AND condition'.
        Expr::BinaryOp { left, op, right } => {
            let left_val = &**left;
            let right_val = &**right;

            match (left_val, right_val) {
                // Evaluate the expressions where right side is a literal value and the left side is a column identifier
                (Expr::Identifier(id), Expr::Value(val)) => {
                    let column = id.value.clone();
                    let value = val.to_string().trim_matches('\'').to_string();
                    match op.to_string().as_str() {
                        "=" => row.get(&column) == Some(&value),
                        "!=" => row.get(&column) != Some(&value),
                        _ => false,
                    }
                }
                // Handle the logical AND & OR operators by recursively evaluating their operands
                _ => {
                    match op.to_string().as_str() {
                        "AND" => evaluate_condition(left_val, row) && evaluate_condition(right_val, row),
                        "OR" => evaluate_condition(left_val, row) || evaluate_condition(right_val, row),
                        _ => false,
                    }
                }
            }
        }
        _ => false,
    }
}

// Next, evaluate a SQL query against a table that is given, by returning the resultant rows and a validity flag
fn evaluate_query(table: &Table, sql: &str) -> (Vec<Row>, bool) {
    let dialect = GenericDialect {};
    // After, attempt to parse a SQL query
    let ast = match Parser::parse_sql(&dialect, sql) {
        Ok(ast) => ast,
        Err(_) => return (vec![], false), // Next, return empty result and false if parsing fails
    };

    // Process the first statement in parsed AST, thereby expecting it to be a Query
    if let Statement::Query(query) = &ast[0] {
        // Make sure that the query body is a 'Select' statement
        if let SetExpr::Select(select) = &*query.body {
            // Basic check: FROM clause cannot be empty
            if select.from.is_empty() {
                return (vec![], false);
            }

            // Verify that the table name in the query matches the table name that is provided
            let table_name_in_query = select.from[0].relation.to_string().to_lowercase();
            if table_name_in_query != table.name.to_lowercase() {
                return (vec![], false);
            }

            let projection = &select.projection;
            let selection = &select.selection;

            // Next, filter the table rows based on 'WHERE' clause, if they are present
            let filtered_rows: Vec<Row> = table
                .rows
                .iter()
                .filter(|row| {
                    if let Some(expr) = selection {
                        evaluate_condition(expr, row) // Use the evaluate_condition function to filter the rows
                    } else {
                        true // If no WHERE clause, include all rows
                    }
                })
         
                .map(|row| {
                    let mut new_row = Row::new();
                    for item in projection {
                        match item {
                            SelectItem::Wildcard(_) => {
                                for (k, v) in row {
                                    new_row.insert(k.clone(), v.clone());
                                }
                            }
                            SelectItem::UnnamedExpr(Expr::Identifier(id)) => {
                                if let Some(val) = row.get(&id.value) {
                                    new_row.insert(id.value.clone(), val.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                    new_row
                })
                .collect();

            return (filtered_rows, true); // Return the result and highlight that it is a valid query
        }
    }

    (vec![], false) // Return the empty result and false for query types that are unsupported
}

fn main() {
    let student_table = Table {
        name: "student".to_string(),
        rows: vec![
            hashmap! {"id".to_string() => "1".to_string(), "name".to_string() => "Alice".to_string(), "major".to_string() => "CS".to_string()},
            hashmap! {"id".to_string() => "2".to_string(), "name".to_string() => "Bob".to_string(), "major".to_string() => "Math".to_string()},
            hashmap! {"id".to_string() => "3".to_string(), "name".to_string() => "Charlie".to_string(), "major".to_string() => "CS".to_string()},
        ],
    };

    println!("Enter your SQL query:");
    print!("> ");
    io::stdout().flush().unwrap();

    let mut sql_input = String::new();
    io::stdin().read_line(&mut sql_input).expect("Failed to read input");
    let sql_input = sql_input.trim();

    let (result, is_valid) = evaluate_query(&student_table, sql_input);

    println!("\nQuery Output:");
    for row in &result {
        println!("{:?}", row);
    }

    println!("\n{} row(s) returned.", result.len());

    if is_valid {
        println!("\nQuery is correct");
    } else {
        println!("\nQuery is incorrect");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> Table {
        Table {
            name: "student".to_string(),
            rows: vec![
                hashmap! {"id".to_string() => "1".to_string(), "name".to_string() => "Alice".to_string(), "major".to_string() => "CS".to_string()},
                hashmap! {"id".to_string() => "2".to_string(), "name".to_string() => "Bob".to_string(), "major".to_string() => "Math".to_string()},
                hashmap! {"id".to_string() => "3".to_string(), "name".to_string() => "Charlie".to_string(), "major".to_string() => "CS".to_string()},
            ],
        }
    }

    //Unit tests are been given to validate the SQL queries

    #[test]
    fn test_case_1_select_star() {
        let (res, valid) = evaluate_query(&sample_table(), "SELECT * FROM student;");
        assert!(valid);
        assert_eq!(res.len(), 3);
    }

    #[test]
    fn test_case_2_select_major() {
        let (res, valid) = evaluate_query(&sample_table(), "SELECT major FROM student;");
        assert!(valid);
        assert_eq!(res.len(), 3);
        assert!(res.iter().all(|r| r.contains_key("major")));
    }

    #[test]
    fn test_case_3_where_major_cs() {
        let (res, valid) = evaluate_query(&sample_table(), "SELECT * FROM student WHERE major = 'CS';");
        assert!(valid);
        assert_eq!(res.len(), 2);
    }

    #[test]
    fn test_case_4_where_major_math() {
        let (res, valid) = evaluate_query(&sample_table(), "SELECT * FROM student WHERE major = 'Math';");
        assert!(valid);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0]["name"], "Bob");
    }

    #[test]
    fn test_case_5_where_name_alice() {
        let (res, valid) = evaluate_query(&sample_table(), "SELECT id, major FROM student WHERE name = 'Alice';");
        assert!(valid);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0]["id"], "1");
        assert_eq!(res[0]["major"], "CS");
    }

    #[test]
    fn test_case_6_invalid_string_literal() {
        let (res, valid) = evaluate_query(&sample_table(), "SELECT name WHERE major = Math;");
        assert!(!valid);
        assert_eq!(res.len(), 0);
    }

    #[test]
    fn test_case_7_nonexistent_column() {
        let (res, valid) = evaluate_query(&sample_table(), "SELECT age FROM student;");
        assert!(valid); 
        assert_eq!(res.len(), 3);
        assert!(res.iter().all(|r| r.is_empty()));
    }

    #[test]
    fn test_case_8_missing_select_clause() {
        let (res, valid) = evaluate_query(&sample_table(), "WHERE major = 'CS';");
        assert!(!valid);
        assert_eq!(res.len(), 0);
    }

    #[test]
    fn test_case_9_and_condition_match() {
        let (res, valid) = evaluate_query(&sample_table(), "SELECT * FROM student WHERE major = 'CS' AND id = '1';");
        assert!(valid);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0]["name"], "Alice");
    }

    #[test]
    fn test_case_10_and_condition_multiple_fields() {
        let (res, valid) = evaluate_query(&sample_table(), "SELECT id, major FROM student WHERE name = 'Charlie' AND major = 'CS';");
        assert!(valid);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0]["id"], "3");
        assert_eq!(res[0]["major"], "CS");
    }
}
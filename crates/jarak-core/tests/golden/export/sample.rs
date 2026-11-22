//! A sample for the export goldens: <tags> & "quotes".
use std::collections::HashMap;

/// 한글 주석과 문자열.
fn main() {
    let mut m: HashMap<&str, u32> = HashMap::new();
    m.insert("한글", 42);
    if m.len() > 0 && true {
        println!("{} {{braces}} \ done", m["한글"]);
    }
}

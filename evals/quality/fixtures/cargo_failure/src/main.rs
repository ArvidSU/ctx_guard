fn parse_port(raw: &str) -> i32 {
    // Intentional type mismatch: assigning a string slice to an i32 triggers a compile error.
    let port: i32 = raw;
    port + 1
}

fn main() {
    let port = parse_port("8080");
    println!("Next port is {}", port);
}


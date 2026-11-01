use std::io::{self, Write as _};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    match mathematishia_cli::run(&borrowed) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            let _ = writeln!(io::stderr(), "error: {error}");
            std::process::exit(2);
        }
    }
}

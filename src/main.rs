use std::process::exit;

use anyhow::Error;

mod bus;

fn main() {
    let bus = match bus::prepare() {
        Ok(value) => value,
        Err(error) => exit_with_error(exitcode::OSERR, error),
    };
}

fn exit_with_error(exitcode: i32, error: Error) -> ! {
    // Reason
    eprintln!("An error has occurred; Reason:\n\t{}\n\nStacktrace:", error);

    // Stacktrace
    for sub_error in error.chain().skip(1) {
        eprintln!("\t- {}", sub_error);
    }

    exit(exitcode)
}

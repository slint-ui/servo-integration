use std::io::Write;

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub fn print_time(str: &str, color: Color) {
    let now = time_now::now_as_millis();
    let last_6_digits = now % 1_000_000;

    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // Print the string in the specified color
    stdout
        .set_color(ColorSpec::new().set_fg(Some(color)))
        .unwrap();
    write!(&mut stdout, "{:<30} {} ms", str, last_6_digits).unwrap();

    // Reset color and add newline
    stdout.reset().unwrap();
    writeln!(&mut stdout).unwrap();
}

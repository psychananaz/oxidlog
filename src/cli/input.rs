use std::io::{self, Write};

pub fn get_input(prompt: &str) -> io::Result<String> {
    let mut output = io::stdout().lock();
    write!(output, "{prompt}")?;
    output.flush()?;
    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Input ended; edit cancelled",
        ));
    }
    Ok(input.trim().to_owned())
}

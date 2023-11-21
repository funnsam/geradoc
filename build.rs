use std::process::Command;
use std::io;
use std::fs;
use std::path::Path;

// https://stackoverflow.com/a/64535181
fn is_input_file_outdated<P1, P2>(input: P1, output: P2) -> io::Result<bool>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let out_meta = fs::metadata(output);
    if let Ok(meta) = out_meta {
        let output_mtime = meta.modified()?;

        // if input file is more recent than our output, we are outdated
        let input_meta = fs::metadata(input)?;
        let input_mtime = input_meta.modified()?;

        Ok(input_mtime > output_mtime)
    } else {
        // output file not found, we are outdated
        Ok(true)
    }
}

fn main() {
    if is_input_file_outdated("src/style.css", "src/style.css.min").unwrap_or(true) {
        assert!(Command::new("uglifycss")
            .args([
                "src/style.css",
                "--output", "src/style.css.min",
            ])
            .spawn()
            .unwrap()
            .wait()
            .unwrap()
            .success()
        );
    }
}

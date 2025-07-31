use std::{fs::File, io::Write};

use jane_eyre::eyre::{self, Context, bail};

/// <https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands#setting-an-output-parameter>
pub fn set_output_parameter(name: impl AsRef<str>, value: impl ToString) -> eyre::Result<()> {
    let name = name.as_ref();
    let value = value.to_string();
    if name.contains(['=', '\n']) || value.contains('\n') || value.starts_with("<<") {
        bail!("Invalid name or value");
    }
    let path = std::env::var("GITHUB_OUTPUT").wrap_err("GITHUB_OUTPUT")?;
    let mut file = File::options().create(true).append(true).open(path)?;
    writeln!(file, "{name}={value}")?;

    Ok(())
}

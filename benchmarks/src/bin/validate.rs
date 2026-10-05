use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let output = match (arguments.next(), arguments.next(), arguments.next()) {
        (None, None, None) => None,
        (Some(flag), Some(path), None) if flag == "--output" => Some(PathBuf::from(path)),
        _ => return Err("usage: validate [--output PATH]".into()),
    };
    let fixtures = html_benchmarks::fixtures()?;
    let rows = html_benchmarks::validate(&fixtures)?;
    let json = serde_json::to_string_pretty(&rows)?;
    if let Some(path) = output {
        std::fs::write(path, format!("{json}\n"))?;
    } else {
        println!("{json}");
    }
    let eligible = rows.iter().filter(|row| row.eligible).count();
    eprintln!(
        "{eligible}/{} workload/fixture/parser pairs produce equivalent link records",
        rows.len()
    );
    Ok(())
}

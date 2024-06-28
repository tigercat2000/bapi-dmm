use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use tempfile::TempDir;

#[test]
fn test_byondapi_with_dreamdaemon() {
    let tempdir = tempfile::tempdir().expect("Failed to create temporary directory");
    let dll = build_dylib();
    copy_to_tmp(&dll, &tempdir);
    compile(&tempdir);

    let stderr = run_dreamdaemon(&tempdir);

    let rust_errored = check_output_rust(&tempdir);
    let dd_errored = check_output_dd(&tempdir);

    if rust_errored || dd_errored {
        panic!("Error logs were produced!");
    }

    #[cfg(unix)]
    const LINE_COUNT: usize = 3;
    #[cfg(windows)]
    const LINE_COUNT: usize = 5;

    if stderr.lines().count() > LINE_COUNT {
        panic!("Stderr contains more than 3 lines, an error message might be printed!")
    }
}

fn bin_path() -> PathBuf {
    match std::env::var("BYOND_LOCATION") {
        Ok(value) => {
            println!("Using byond from dir {value}");
            value.into()
        }
        Err(_) => {
            println!("Byond not found, using default location");
            println!("To set a location for byond, set the BYOND_LOCATION environment variable to a path");
            println!("Keep in mind that this path has to point to the /bin folder of byond");
            "C:\\Program Files (x86)\\BYOND\\bin".into()
        }
    }
}

fn find_dm() -> PathBuf {
    if cfg!(windows) {
        bin_path().join("dm.exe")
    } else {
        "DreamMaker".into()
    }
}

fn find_dd() -> PathBuf {
    if cfg!(windows) {
        bin_path().join("dd.exe")
    } else {
        "DreamDaemon".into()
    }
}

fn build_dylib() -> PathBuf {
    let mut cmd = Command::new(option_env!("CARGO").unwrap_or("cargo"));

    cmd.arg("build").arg("--message-format=json").arg("--lib");
    #[cfg(windows)]
    cmd.arg("--target=i686-pc-windows-msvc");
    #[cfg(unix)]
    cmd.arg("--target=i686-unknown-linux-gnu");
    cmd.stderr(std::process::Stdio::inherit());
    parse_output(cmd.output().unwrap())
}

fn parse_output(res: Output) -> PathBuf {
    let mut artifact = None;
    for message in cargo_metadata::Message::parse_stream(res.stdout.as_slice()) {
        match message.unwrap() {
            cargo_metadata::Message::CompilerMessage(m) => eprintln!("{}", m),
            cargo_metadata::Message::CompilerArtifact(a) => artifact = Some(a),
            _ => (),
        }
    }

    if !res.status.success() {
        panic!("Failed to build")
    }
    artifact.unwrap().filenames[0].clone().into()
}

fn compile(tempdir: &TempDir) {
    let dm_compiler = find_dm();

    let output = Command::new(dm_compiler)
        .current_dir(tempdir)
        .arg("test.dme")
        .output()
        .expect("Failed to compile DM project");

    assert!(
        tempdir.path().join("test.dmb").exists(),
        "test.dmb was not created: {:#?}",
        output
    );
}

fn copy_to_tmp(dll: &Path, tempdir: &TempDir) {
    let target = tempdir.path();

    let dm_origin = Path::new(env! {"CARGO_MANIFEST_DIR"}).join("dm");
    for file in dm_origin.read_dir().expect("Failed to copy dm files") {
        let _ = file
            .map(|f| std::fs::copy(f.path(), target.join(f.file_name())))
            .expect("Failed to copy file");
    }

    let test_origin = Path::new(env! {"CARGO_MANIFEST_DIR"})
        .join("tests")
        .join("test_project");
    for file in test_origin.read_dir().expect("Failed to copy dm files") {
        let _ = file
            .map(|f| std::fs::copy(f.path(), target.join(f.file_name())))
            .expect("Failed to copy file");
    }

    std::fs::copy(dll, target.join("bapi_dmm_reader.dll"))
        .expect("Failed to copy bapi_dmm_reader.dll");
}

fn run_dreamdaemon(tempdir: &TempDir) -> String {
    let dream_daemon = find_dd();

    let dd_output = Command::new(dream_daemon)
        .current_dir(tempdir.path())
        .arg("test.dmb")
        .arg("-trusted")
        .output()
        .expect("DreamDaemon crashed");
    let stdout = std::str::from_utf8(&dd_output.stdout).unwrap();
    let stderr = std::str::from_utf8(&dd_output.stderr).unwrap();
    if !stdout.is_empty() {
        eprintln!("Stdout:-------------------------------------------------------------------");
        eprintln!("{stdout}");
    }

    if !stderr.is_empty() {
        eprintln!("Stderr:-------------------------------------------------------------------");
        eprintln!("{stderr}");
    }
    stderr.to_owned()
}

fn check_output_dd(tempdir: &TempDir) -> bool {
    let log = tempdir.path().join("dd_log.txt");

    assert!(log.exists(), "The test did not produce any output");

    let log = std::fs::read_to_string(log).expect("Failed to read log");

    eprintln!("DDlogs:-------------------------------------------------------------------");
    eprintln!("{}", log);

    log.contains("runtime error:")
}

fn check_output_rust(tempdir: &TempDir) -> bool {
    let log = tempdir.path().join("rust_log.txt");

    if log.exists() {
        let log = std::fs::read_to_string(log).expect("Failed to read log");
        eprintln!("Rustlogs:-----------------------------------------------------------------");
        eprintln!("{}", log);
        true
    } else {
        false
    }
}

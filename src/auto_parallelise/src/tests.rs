use std::path::Path;
use std::process::Command;
use utils;
use SAVE_FILE;
use CONFIG_FILE;
use plugin::shared_state::Config;
use std::fs;

/// Removes any files from previous compiles
fn clean(folder: &Path) {
    Command::new("cargo").arg("clean")
            .current_dir(&folder)
            .output().expect("Unable to run cargo clean");
    Command::new("rm").arg(SAVE_FILE)
            .current_dir(&folder)
            .output().expect(&format!("Unable to rm {}", SAVE_FILE));
    Command::new("rm").arg(CONFIG_FILE)
            .current_dir(&folder)
            .output().expect(&format!("Unable to rm {}", CONFIG_FILE));
}

fn compile(build_config: &Config, folder: &Path) -> String {
    // Remove the .autoparallelise file
    Command::new("rm").arg(SAVE_FILE)
            .current_dir(&folder)
            .output().expect(&format!("Unable to rm {}", SAVE_FILE));

    // Set the config
    let mut config_path_buf = folder.to_path_buf();
    config_path_buf.push(CONFIG_FILE);
    let config_path = config_path_buf.as_path();
    build_config.save(config_path);

    // Compile first stage: Analysis
    let stage1output = Command::new("cargo").arg("build")
                               .current_dir(&folder)
                               .output().expect("Unable to compile analysis stage");
     println!("Stage 1 Output:\n{}\n\n\n", String::from_utf8_lossy(&stage1output.stderr));

    // Compile second stage: Modification
    let stage2output = Command::new("cargo")
                               .arg("build")
                               .current_dir(&folder)
                               .output().expect("Unable to compile modification stage");
    println!("Stage 2 Output:\n{}", String::from_utf8_lossy(&stage2output.stderr));

    // Return stdout which contains the parallelised source code (if enabled)
    return format!("{}", String::from_utf8_lossy(&stage2output.stdout));
}

fn create_tmpfolder() -> String {
    let mktempoutput = Command::new("mktemp").arg("-d").arg("-p").arg("../test_folder")
                               .output().expect("Unable to create temp directory");
    let mut foldername = format!("{}", String::from_utf8_lossy(&mktempoutput.stdout));
    let truncate_amount = foldername.len() - 1;
    foldername.truncate(truncate_amount);
    foldername
}


fn run(path: &Path) -> String {
    let cmdoutput = Command::new("cargo")
            .arg("run")
            .current_dir(&path)
            .output()
            .expect(&format!("Unable to run {}", path.display()));
    format!("{}", String::from_utf8_lossy(&cmdoutput.stdout))
}

fn cleanup(path: &Path) {

}


fn folder_code_and_run(parallel_code: &String, sequential_path: &Path) -> String {
    // Create temp folder
    let parallel_folder = create_tmpfolder();
    fs::create_dir_all(format!("{}/src/", parallel_folder));
    println!("parallel_folder: {}", parallel_folder);
    let parallel_path = Path::new(&parallel_folder);

    // Copy Cargo.toml into new folder
    Command::new("cp").arg("Cargo.toml").arg(format!("{}/Cargo.toml", parallel_folder))
            .current_dir(&sequential_path)
            .output().expect("Unable to copy cargo.toml");

    // Read in original source code to extract imports
    let original_source = utils::read_file(&format!("{}/src/main.rs", sequential_path.display())).unwrap();
    let mut import_str = original_source.lines().filter(|line| {
        line.starts_with("use") || line.starts_with("extern") ||
        (line.starts_with("#!") && !line.contains("plugin"))
    }).fold("".to_owned(), |acc, x| {
        let mut acc2 = acc.clone();
        acc2.push_str(&x);
        acc2.push_str("\n");
        acc2
    });

    // Combine imports and paralell code
    let mut parallel_code_with_imports = import_str;
    parallel_code_with_imports.push_str(parallel_code);

    // Save the code to src/main.rs
    let mut pathbuf = parallel_path.to_path_buf();
    pathbuf.push("src/main.rs");
    let parallel_code_path = pathbuf.as_path();
    utils::write_file(&parallel_code_path, &parallel_code_with_imports);

    // Compile with plugin disabled and run
    let mut build_config = Config::default();
    build_config.plugin_enabled = false;
    compile(&build_config, &parallel_path);
    run(&parallel_path)

    // Cleanup path

}

fn compare_outputs(sequential_output: &String, parallel_output: &String) {
    let mut seqlines: Vec<&str> = sequential_output.lines().collect();
    let mut parlines: Vec<&str> = parallel_output.lines().collect();
    seqlines.sort_unstable();
    parlines.sort_unstable();
    assert!(seqlines.len() == parlines.len(), "Outputs have different lengths");
    for i in 0..seqlines.len() {
        let seqline = seqlines[i];
        let parline = parlines[i];
        assert!(seqline == parline, "{} != {}", seqline, parline);
    }
}

fn test_foldered_program(folder: &str) {
    let path = Path::new(&folder);

    clean(&path);

    // Configure build options
    let mut build_config = Config::default();

    // Parallel build
    println!("Parallel Build");
    build_config.plugin_enabled = true;
    let parallel_code = compile(&build_config, path.clone());
    let parallel_output = folder_code_and_run(&parallel_code, &path);

    // Sequential build
    println!("Sequential Build");
    build_config.plugin_enabled = false;
    compile(&build_config, &path);
    let sequential_output = run(&path);


    // Compare parallel_ouptut and sequential_output to make sure they are the same
    println!("Sequential Output:\n{}", sequential_output);
    println!("Parallel Output:\n{}", parallel_output);
    compare_outputs(&sequential_output, &parallel_output);
}


#[test]
fn password_simple() { test_foldered_program("../password-simple") }
#[test]
fn simple_example() { test_foldered_program("../simple-example") }
#[test]
fn fibinacci() { test_foldered_program("../fibinacci") }

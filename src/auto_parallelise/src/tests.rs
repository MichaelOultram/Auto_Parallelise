use std::path::Path;
use std::process::Command;
use SAVE_FILE;
use CONFIG_FILE;
use plugin::shared_state::Config;

fn compile(build_config: &Config, folder: &Path) -> String {
    // Remove the .autoparallelise file
    Command::new("rm")
            .arg(SAVE_FILE)
            .current_dir(&folder)
            .output();

    // Set the config
    let mut config_path_buf = folder.to_path_buf();
    config_path_buf.push(CONFIG_FILE);
    let config_path = config_path_buf.as_path();
    build_config.save(config_path);

    // Compile first stage: Analysis
    let stage1output = Command::new("cargo")
                               .arg("build")
                               .current_dir(&folder)
                               .output()
                               .expect("Unable to compile analysis stage");
     println!("{}\n\n\n", String::from_utf8_lossy(&stage1output.stderr));

    // Compile second stage: Modification
    let stage2output = Command::new("cargo")
                               .arg("build")
                               .current_dir(&folder)
                               .output()
                               .expect("Unable to compile modification stage");
    println!("{}", String::from_utf8_lossy(&stage2output.stderr));
    return format!("{}", String::from_utf8_lossy(&stage2output.stdout));
}


fn test_manual_program(folder: &str) {
    let relpath_str = format!("../{}", folder);
    let path = Path::new(&relpath_str);

    let mut build_config = Config::default();

    build_config.enabled = true;
    let parallel_code = compile(&build_config, path.clone());

    build_config.enabled = false;
    let sequential_code = compile(&build_config, path.clone());

    println!("Sequential Code:\n{}", sequential_code);
    println!("Parallel Code:\n{}", parallel_code);
}


#[test]
fn password_simple() {
    test_manual_program("password-simple");

    panic!()
}

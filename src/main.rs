fn main() {
    if let Err(_) = std::env::var("BOTTLED_SHELL_LOG") {
        std::env::set_var("BOTTLED_SHELL_LOG", "info");
    }
    pretty_env_logger::init_custom_env("BOTTLED_SHELL_LOG");

    let mut app = clap::App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .about("launch bottled shell. replacing login shell")
        .setting(clap::AppSettings::TrailingVarArg)
        .setting(clap::AppSettings::DontDelimitTrailingValues)
        .setting(clap::AppSettings::AllowLeadingHyphen)
        .setting(clap::AppSettings::DisableVersion)
        .arg(
            clap::Arg::with_name("shell-options")
                .multiple(true)
                .allow_hyphen_values(true)
        );
    let matches = app.get_matches_from_safe_borrow(std::env::args_os()).unwrap_or_else(|e| {
        if e.use_stderr() {
            eprintln!("{}", e.message);
            std::process::exit(1);
        }
        e.exit()
    });

    for (key, value) in std::env::vars() {
        log::debug!("env {}: {}", key, value);
    }

    let bottled_cmd = if let Some((c, _)) = clap::crate_name!().rsplit_once('-') {
        c.clone()
    } else {
        clap::crate_name!()
    };
    let bottled_cmd_path = std::env::current_exe().unwrap()
        .parent().unwrap()
        .join(bottled_cmd)
        .to_str().unwrap()
        .to_string();
    let mut shell = "bash";
    if let Some(b) = app.get_bin_name() {
        if let Some((_, s)) = b.rsplit_once('-') {
            shell = s.clone();
        }
    }
    let mut args: Vec<std::ffi::CString> = vec![
        std::ffi::CString::new(bottled_cmd_path.clone()).unwrap(),
        std::ffi::CString::new("shell").unwrap(),
        std::ffi::CString::new("-s").unwrap(),
        std::ffi::CString::new(shell).unwrap(),
    ];
    if matches.is_present("shell-options") {
        args.push(std::ffi::CString::new("--").unwrap());
        for v in matches.values_of_lossy("shell-options").unwrap() {
            args.push(std::ffi::CString::new(v.as_str()).unwrap());
        }
    }

    log::trace!("executing bottled shell: {:?}", args);
    nix::unistd::execv(
        &std::ffi::CString::new(bottled_cmd_path.as_str()).unwrap().as_c_str(),
        &args
    ).unwrap();
}
